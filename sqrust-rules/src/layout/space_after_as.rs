use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SpaceAfterAs;

impl Rule for SpaceAfterAs {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterAs"
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
    let kw = b"AS";
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

        // Skip positions inside strings or comments.
        if skip[i] {
            i += 1;
            continue;
        }

        // Need at least kw_len bytes remaining to match AS.
        if i + kw_len > len {
            i += 1;
            continue;
        }

        // Word-boundary check before AS: char before A must NOT be alphanumeric or '_'.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            // Word-boundary check after AS: char right after S must not be alphanumeric/underscore
            // for a standalone AS keyword (no word boundary after → part of longer word like CASE, CAST).
            let after = i + kw_len;
            // First ensure AS is a complete word: the character after S must be a non-word char
            // OR it must be a word char (in which case it's a violation — no space after AS).
            // But we also need to confirm AS is not part of a longer word at the end,
            // e.g. "ALIAS" — the 'A' at pos i: before char is 'L' (word char) → before_ok fails.
            // We only reach here when before_ok is true.
            if after < len && is_word_char(bytes[after]) && !skip[after] {
                // AS immediately followed by word char with no space — violation.
                let col = i - line_start + 1;
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "Missing space after AS — write 'AS alias' not 'ASalias'".to_string(),
                    line,
                    col,
                });
                i += kw_len;
                continue;
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
