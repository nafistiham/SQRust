use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CommentStyle;

impl Rule for CommentStyle {
    fn name(&self) -> &'static str {
        "Layout/CommentStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut i = 0usize;
    let mut in_string = false;

    while i < len {
        let byte = bytes[i];

        // ── String tracking ────────────────────────────────────────────────
        if in_string {
            if byte == b'\'' {
                // SQL '' escape: two consecutive single-quotes inside a string
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Enter single-quoted string
        if byte == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }

        // ── Skip -- line comment (to end of line) ─────────────────────────
        if i + 1 < len && byte == b'-' && bytes[i + 1] == b'-' {
            // Advance past the entire line so we don't misidentify anything inside
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // ── Block comment /* ... */ ────────────────────────────────────────
        if i + 1 < len && byte == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2; // move past /*

            let mut has_newline = false;
            let mut closed = false;

            while i < len {
                if bytes[i] == b'\n' {
                    has_newline = true;
                }
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2; // move past */
                    closed = true;
                    break;
                }
                i += 1;
            }

            // Unclosed block comment: treat as single-line (no newline found)
            if !has_newline || !closed {
                // Only flag if there was no newline (i.e. single-line usage)
                if !has_newline {
                    let (line, col) = byte_offset_to_line_col(source, start);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message:
                            "Single-line /* */ comment; use -- for single-line comments"
                                .to_string(),
                        line,
                        col,
                    });
                }
            }

            continue;
        }

        i += 1;
    }

    diags
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
