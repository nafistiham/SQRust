use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoSpaceAfterUnaryMinus;

/// Characters that, when immediately preceding `-`, indicate a unary context.
/// A word character or digit before `-` means binary minus (e.g. `5 - col`).
const UNARY_TRIGGER: &[u8] = b"(=<>!,+*/";

impl Rule for NoSpaceAfterUnaryMinus {
    fn name(&self) -> &'static str {
        "Layout/NoSpaceAfterUnaryMinus"
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
    let mut line: usize = 1;
    let mut line_start: usize = 0;

    for i in 0..len {
        let b = bytes[i];

        if b == b'\n' {
            line += 1;
            line_start = i + 1;
            continue;
        }

        // Only examine `-` that is in code (not inside string/comment).
        if b != b'-' || skip[i] {
            continue;
        }

        // The non-whitespace character before the `-` (skipping spaces/tabs)
        // must be a unary trigger char. A word char or digit before `-`
        // indicates binary minus (e.g. `5 - col`), so we skip those.
        if i == 0 {
            continue;
        }
        let prev_non_ws = find_prev_non_ws(bytes, i);
        match prev_non_ws {
            Some(prev) if UNARY_TRIGGER.contains(&prev) => {}
            _ => continue,
        }

        // The character immediately after must be a space.
        if i + 1 >= len || bytes[i + 1] != b' ' {
            continue;
        }

        let col = i - line_start + 1;
        diags.push(Diagnostic {
            rule: rule_name,
            message: "Space after unary minus operator — write '-expr' not '- expr'".to_string(),
            line,
            col,
        });
    }

    diags
}

/// Return the first non-space/non-tab byte before position `pos`, or `None`
/// if only whitespace (or nothing) precedes it.
fn find_prev_non_ws(bytes: &[u8], pos: usize) -> Option<u8> {
    let mut j = pos;
    while j > 0 {
        j -= 1;
        if bytes[j] != b' ' && bytes[j] != b'\t' {
            return Some(bytes[j]);
        }
    }
    None
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
