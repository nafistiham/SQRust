use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CommentWithoutSpace;

impl Rule for CommentWithoutSpace {
    fn name(&self) -> &'static str {
        "Lint/CommentWithoutSpace"
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

    // Build a skip set so we ignore comment/string content when scanning for
    // patterns inside strings.  We do NOT mark comments themselves as skipped
    // here — instead we scan them deliberately below.
    let in_string = build_string_skip_set(bytes, len);

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

        // Skip bytes that are inside a string literal.
        if in_string[i] {
            i += 1;
            continue;
        }

        // Check for block comment start: /*
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Advance past /*
            let comment_start = i;
            i += 2;

            // Check character immediately after /*
            // Exceptions: */ (empty comment /**/ or /**/), space, newline
            if i < len {
                let next = bytes[i];
                let is_close_star = next == b'*' && i + 1 < len && bytes[i + 1] == b'/';
                if next != b' ' && next != b'\t' && next != b'\n' && next != b'\r' && !is_close_star {
                    let col = comment_start - line_start + 1;
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "Block comment '/*' should be followed by a space — write '/* comment */' not '/*comment*/'".to_string(),
                        line,
                        col,
                    });
                }
            }

            // Skip to end of block comment so we don't double-flag
            while i < len {
                if bytes[i] == b'\n' {
                    line += 1;
                    line_start = i + 1;
                }
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Check for line comment start: --
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let comment_start = i;
            i += 2;

            // Exception: --- or more dashes (decorative separator) — skip
            if i < len && bytes[i] == b'-' {
                // Triple-dash or longer: skip to end of line without flagging
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Check character immediately after --
            // OK: end of input, newline (empty comment), space, tab
            if i < len {
                let next = bytes[i];
                if next != b' ' && next != b'\t' && next != b'\n' && next != b'\r' {
                    let col = comment_start - line_start + 1;
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "Line comment '--' should be followed by a space — write '-- comment' not '--comment'".to_string(),
                        line,
                        col,
                    });
                }
            }

            // Skip to end of line
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    diags
}

/// Build a boolean set where `in_string[i] == true` means byte `i` is inside
/// a single-quoted or double-quoted literal.  Block and line comments are NOT
/// marked — we handle them in the main loop above.
fn build_string_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
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

        // Skip over block comments without marking them as "string" — the
        // main loop handles them.  We just need to advance past them so the
        // string scanner doesn't mistakenly consume a quote inside a comment.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Skip over line comments.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
