use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ParenthesisSpacing;

impl Rule for ParenthesisSpacing {
    fn name(&self) -> &'static str {
        "ParenthesisSpacing"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let violations = find_violations(&ctx.source, self.name());
        if violations.is_empty() {
            return None;
        }

        let bytes = ctx.source.as_bytes();
        let len = bytes.len();
        let skip = build_skip_set(bytes, len);

        let mut result: Vec<u8> = Vec::with_capacity(len);
        let mut i = 0;

        while i < len {
            let ch = bytes[i];

            // Remove space(s) immediately after `(` (but not newlines)
            if ch == b'(' {
                result.push(ch);
                i += 1;
                // Skip any spaces that follow (not newlines)
                while i < len && bytes[i] == b' ' && !skip[i] {
                    i += 1;
                }
                continue;
            }

            // Remove space(s) immediately before `)`
            // We need to look ahead: if current byte is space and next non-space is `)`, drop spaces
            if ch == b' ' && !skip[i] {
                // Scan forward to see if there's a `)` after only spaces
                let mut j = i;
                while j < len && bytes[j] == b' ' && !skip[j] {
                    j += 1;
                }
                if j < len && bytes[j] == b')' {
                    // Suppress all these spaces; let `)` be emitted on next iteration
                    i = j;
                    continue;
                } else {
                    result.push(ch);
                    i += 1;
                    continue;
                }
            }

            result.push(ch);
            i += 1;
        }

        Some(String::from_utf8(result).expect("source was valid UTF-8"))
    }
}

/// Scans the source for paren spacing violations.
fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip = build_skip_set(bytes, len);

    let mut diags = Vec::new();

    for i in 0..len {
        // Space after opening parenthesis: `(` immediately followed by ` `
        if i + 1 < len
            && bytes[i] == b'('
            && bytes[i + 1] == b' '
            && !skip[i + 1]
        {
            let (line, col) = byte_offset_to_line_col(source, i + 1);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Space after opening parenthesis; remove the space".to_string(),
                line,
                col,
            });
        }

        // Space before closing parenthesis: ` ` immediately followed by `)`
        if i + 1 < len
            && bytes[i] == b' '
            && bytes[i + 1] == b')'
            && !skip[i]
        {
            let (line, col) = byte_offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Space before closing parenthesis; remove the space".to_string(),
                line,
                col,
            });
        }
    }

    diags
}

/// Builds a boolean skip-set (indexed by byte offset).
/// A byte is in the skip set if it lies inside:
///   - a single-quoted string `'...'` (with `''` escaping)
///   - a double-quoted identifier `"..."` (with `""` escaping)
///   - a block comment `/* ... */`
///   - a line comment `-- ...` (until newline)
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string
        if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        // Escaped quote `''`
                        skip[start..=i + 1].fill(true);
                        i += 2;
                        continue;
                    }
                    // Closing quote
                    skip[start..=i].fill(true);
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        // Escaped quote `""`
                        skip[start..=i + 1].fill(true);
                        i += 2;
                        continue;
                    }
                    // Closing quote
                    skip[start..=i].fill(true);
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment `/* ... */`
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[start..=i + 1].fill(true);
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment `-- ...`
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            skip[start..i].fill(true);
            continue;
        }

        i += 1;
    }

    skip
}

/// Converts a byte offset into a 1-indexed (line, col) pair.
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = offset - line_start + 1;
    (line, col)
}
