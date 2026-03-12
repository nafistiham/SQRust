use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NullableConcat;

/// Null-guard function names (uppercase for comparison).
const NULL_GUARDS: &[&str] = &["COALESCE", "IFNULL", "ISNULL", "NVL"];

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` at every byte inside strings, comments, or
/// quoted identifiers.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... newline
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i] = true;
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' escape
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    if i < len && bytes[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'"' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'`' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}

/// Returns true if the source slice (a window ending just before the `||`) is
/// guarded by a null-guard function. We look backwards up to 50 chars for the
/// opening `(` of a null-guard call, scanning past spaces and alphanumeric/`_`/`.`
/// chars. A closing `)` immediately before the `||` means the left operand was
/// wrapped in some function; we then scan back to find the function name.
fn left_side_guarded(source: &str, skip: &[bool], op_offset: usize) -> bool {
    if op_offset == 0 {
        return false;
    }
    let bytes = source.as_bytes();

    // Walk left, skipping whitespace, to find what kind of token ends the left operand.
    let mut i = op_offset as isize - 1;

    // Skip trailing spaces
    while i >= 0 && bytes[i as usize] == b' ' {
        i -= 1;
    }

    if i < 0 {
        return false;
    }

    if bytes[i as usize] == b')' {
        // The left operand is a parenthesised expression. Find the matching '('
        // and the function name before it.
        let close_paren = i as usize;
        // Walk backwards to find matching '(' — track depth
        let mut depth = 1i32;
        i -= 1;
        while i >= 0 && depth > 0 {
            let b = bytes[i as usize];
            if !skip[i as usize] {
                if b == b')' {
                    depth += 1;
                } else if b == b'(' {
                    depth -= 1;
                }
            }
            if depth > 0 {
                i -= 1;
            }
        }
        // i now points at the '('
        if i < 0 {
            return false;
        }
        let open_paren = i as usize;
        // The function name is the word immediately before open_paren
        return function_name_before(source, open_paren, close_paren);
    }

    // If the last token before `||` is a plain identifier/literal, it is bare.
    false
}

/// Returns true if the source slice immediately after `|| ` (starting at
/// `op_offset + 2`) begins with a null-guard function call.
fn right_side_guarded(source: &str, _skip: &[bool], op_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = op_offset + 2; // skip past both `|` chars

    // Skip whitespace
    while i < len && bytes[i] == b' ' {
        i += 1;
    }

    if i >= len {
        return false;
    }

    // Read identifier characters to get the next token
    let start = i;
    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }

    if i >= len || bytes[i] != b'(' {
        // Not a function call — bare identifier or literal
        // A literal starts with a digit or quote; those can't be null
        let token = &source[start..i];
        if token.is_empty() {
            // Could be a quote — treat as literal (not bare column)
            let first = bytes[start];
            return first == b'\'' || first.is_ascii_digit();
        }
        // If token is all digits it's a numeric literal — not nullable
        if token.bytes().all(|b| b.is_ascii_digit()) {
            return true; // treat as "guarded" (it's a literal)
        }
        return false; // bare column
    }

    // It's a function call — check if it's a null guard
    let func_name = source[start..i].to_uppercase();
    NULL_GUARDS.contains(&func_name.as_str())
}

/// Returns true if the token immediately before `open_paren` is a null-guard
/// function name. The `_close_paren` parameter is unused but retained for
/// potential future use.
fn function_name_before(source: &str, open_paren: usize, _close_paren: usize) -> bool {
    if open_paren == 0 {
        return false;
    }
    let bytes = source.as_bytes();
    let mut end = open_paren as isize - 1;

    // Skip whitespace
    while end >= 0 && bytes[end as usize] == b' ' {
        end -= 1;
    }

    if end < 0 {
        return false;
    }

    let name_end = end as usize + 1;
    let mut start = end;
    while start >= 0
        && (bytes[start as usize].is_ascii_alphanumeric() || bytes[start as usize] == b'_')
    {
        start -= 1;
    }
    let name_start = (start + 1) as usize;
    if name_start >= name_end {
        return false;
    }
    let func_name = source[name_start..name_end].to_uppercase();
    NULL_GUARDS.contains(&func_name.as_str())
}

/// Returns true if the left operand of a `||` at `op_offset` appears to be a
/// string literal (ends with `'`) or a numeric literal.
fn left_side_is_literal(source: &str, op_offset: usize) -> bool {
    if op_offset == 0 {
        return false;
    }
    let bytes = source.as_bytes();
    let mut i = op_offset as isize - 1;

    // Skip trailing spaces
    while i >= 0 && bytes[i as usize] == b' ' {
        i -= 1;
    }

    if i < 0 {
        return false;
    }

    let b = bytes[i as usize];
    // Ends with closing quote → string literal
    // Ends with digit → numeric literal
    b == b'\'' || b.is_ascii_digit()
}

/// Returns true if the right operand of a `||` at `op_offset` appears to be a
/// string literal or numeric literal.
fn right_side_is_literal(source: &str, op_offset: usize) -> bool {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = op_offset + 2;

    while i < len && bytes[i] == b' ' {
        i += 1;
    }

    if i >= len {
        return false;
    }

    let b = bytes[i];
    b == b'\'' || b.is_ascii_digit()
}

impl Rule for NullableConcat {
    fn name(&self) -> &'static str {
        "Convention/NullableConcat"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();
        let len = bytes.len();
        let mut i = 0;

        while i + 1 < len {
            if skip[i] {
                i += 1;
                continue;
            }

            if bytes[i] == b'|' && bytes[i + 1] == b'|' && !skip[i + 1] {
                let op_offset = i;

                let left_literal = left_side_is_literal(source, op_offset);
                let right_literal = right_side_is_literal(source, op_offset);

                // Only flag when at least one side is a bare (potentially nullable) column.
                // Literals are not nullable, so skip if both sides are literals.
                if !left_literal && !right_literal {
                    // Both sides could be columns — check for null guards
                    let lg = left_side_guarded(source, &skip, op_offset);
                    let rg = right_side_guarded(source, &skip, op_offset);
                    if !lg || !rg {
                        let (line, col) = line_col(source, op_offset);
                        diags.push(Diagnostic {
                            rule: "Convention/NullableConcat",
                            message: "String concatenation with || may produce NULL if any operand is NULL — consider wrapping columns with COALESCE"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                } else if left_literal && !right_literal {
                    // Left is a literal, right might be a bare column
                    let rg = right_side_guarded(source, &skip, op_offset);
                    if !rg {
                        let (line, col) = line_col(source, op_offset);
                        diags.push(Diagnostic {
                            rule: "Convention/NullableConcat",
                            message: "String concatenation with || may produce NULL if any operand is NULL — consider wrapping columns with COALESCE"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                } else if !left_literal && right_literal {
                    // Right is a literal, left might be a bare column
                    let lg = left_side_guarded(source, &skip, op_offset);
                    if !lg {
                        let (line, col) = line_col(source, op_offset);
                        diags.push(Diagnostic {
                            rule: "Convention/NullableConcat",
                            message: "String concatenation with || may produce NULL if any operand is NULL — consider wrapping columns with COALESCE"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                }
                // both left_literal && right_literal → no violation

                i += 2;
                continue;
            }

            i += 1;
        }

        diags
    }
}
