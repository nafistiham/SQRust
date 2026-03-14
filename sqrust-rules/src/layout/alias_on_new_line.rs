use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct AliasOnNewLine;

impl Rule for AliasOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/AliasOnNewLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();
    let mut i = 0;
    let mut line: usize = 1;
    let mut line_start: usize = 0;

    while i < len {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
            i += 1;
            continue;
        }

        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match `AS` keyword (case-insensitive, word-bounded).
        if i + 2 <= len && bytes[i..i + 2].eq_ignore_ascii_case(b"AS") {
            let after = i + 2;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = after >= len || !is_word_char(bytes[after]);

            if before_ok && after_ok {
                // Check whether AS is the first non-whitespace token on this line.
                // Scan backwards from i along the current line (not past line_start).
                let only_whitespace_before_on_line = (line_start..i)
                    .all(|k| bytes[k] == b' ' || bytes[k] == b'\t');

                if only_whitespace_before_on_line {
                    // AS is at the start of a line (possibly indented).
                    // Now check what the previous line ended with.
                    // Find the character just before this line's newline.
                    // i.e., look backwards from line_start - 1 (the '\n') to find the
                    // last non-whitespace byte on the previous line.
                    if line_start == 0 {
                        // AS is on the very first line at column 1 — nothing before it.
                        i += 2;
                        continue;
                    }

                    // line_start - 1 is the '\n' character; go to line_start - 2.
                    let prev_newline = line_start - 1; // index of the '\n'
                    if prev_newline == 0 {
                        i += 2;
                        continue;
                    }

                    // Find last non-whitespace byte on the previous line.
                    let mut k = prev_newline - 1;
                    while k > 0 && (bytes[k] == b' ' || bytes[k] == b'\t') {
                        k -= 1;
                    }

                    let last_byte = bytes[k];

                    // If preceded by `)` — subquery or CTE body — do not flag.
                    // These are legitimate multi-line patterns:
                    //   ) AS sub_alias
                    //   ) AS (SELECT ...)  — CTE
                    if last_byte == b')' {
                        i += 2;
                        continue;
                    }

                    // If AS is followed by `(` — CTE body definition (`cte_name\nAS (...)`)
                    // — do not flag. We are only targeting table aliases.
                    let mut j = after;
                    while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                        j += 1;
                    }
                    if j < len && bytes[j] == b'(' {
                        i += 2;
                        continue;
                    }

                    // If the position k is in the skip set (inside a string/comment),
                    // skip. This handles edge cases where the previous line is
                    // entirely a comment or string.
                    if skip[k] {
                        i += 2;
                        continue;
                    }

                    // Otherwise: AS is on its own line after an identifier — flag it.
                    let col = i - line_start + 1;
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "Table alias AS keyword is on a new line — place the alias on the same line as the table name".to_string(),
                        line,
                        col,
                    });
                }
            }
        }

        i += 1;
    }

    diags
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
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
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
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

        i += 1;
    }

    skip
}
