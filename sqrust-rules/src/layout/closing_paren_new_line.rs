use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ClosingParenNewLine;

impl Rule for ClosingParenNewLine {
    fn name(&self) -> &'static str {
        "Layout/ClosingParenNewLine"
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

    // Stack of line numbers where each `(` was seen (1-indexed).
    // We push when we see `(` and pop when we see `)`.
    let mut open_paren_lines: Vec<usize> = Vec::new();

    let mut i = 0usize;
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

        if bytes[i] == b'(' {
            open_paren_lines.push(line);
            i += 1;
            continue;
        }

        if bytes[i] == b')' {
            let open_line = open_paren_lines.pop();

            if let Some(open_line) = open_line {
                // Only care about multi-line groups.
                if line != open_line {
                    // Check whether `)` is the first non-whitespace char on its line.
                    let only_ws_before = (line_start..i)
                        .all(|k| bytes[k] == b' ' || bytes[k] == b'\t');

                    if !only_ws_before {
                        let col = i - line_start + 1;
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: "Closing parenthesis of a multi-line expression should be on its own line".to_string(),
                            line,
                            col,
                        });
                    }
                }
            }

            i += 1;
            continue;
        }

        i += 1;
    }

    diags
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
