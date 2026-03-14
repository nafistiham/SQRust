use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SelectStarSpacing;

impl Rule for SelectStarSpacing {
    fn name(&self) -> &'static str {
        "Layout/SelectStarSpacing"
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
    let kw = b"SELECT";
    let kw_len = kw.len();
    let mut diags = Vec::new();
    let mut line: usize = 1;
    let mut line_start: usize = 0;
    let mut i = 0;

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

        // Need at least kw_len bytes remaining.
        if i + kw_len > len {
            i += 1;
            continue;
        }

        // Word-boundary check before SELECT.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            let after = i + kw_len;
            // Word-boundary check after SELECT.
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok && after < len {
                // Pattern 1: SELECT immediately followed by * (no space).
                if bytes[after] == b'*' && !skip[after] {
                    let col = i - line_start + 1;
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "No space between SELECT and * — write 'SELECT *' with exactly one space".to_string(),
                        line,
                        col,
                    });
                    i += kw_len;
                    continue;
                }

                // Pattern 2: SELECT followed by 2+ spaces then *.
                if bytes[after] == b' ' {
                    // Count spaces.
                    let mut space_end = after;
                    while space_end < len && bytes[space_end] == b' ' {
                        space_end += 1;
                    }
                    let space_count = space_end - after;
                    if space_count >= 2 && space_end < len && bytes[space_end] == b'*' && !skip[space_end] {
                        let col = i - line_start + 1;
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: "Multiple spaces between SELECT and * — write 'SELECT *' with exactly one space".to_string(),
                            line,
                            col,
                        });
                        i += kw_len;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
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
