use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SpaceAroundConcatOperator;

impl Rule for SpaceAroundConcatOperator {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundConcatOperator"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let src = &ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();

        let mut diags = Vec::new();

        let mut in_single_quote = false;
        let mut in_block_comment = false;
        let mut in_line_comment = false;

        let mut i = 0;
        while i < len {
            // Handle single-quoted string interior
            if in_single_quote {
                if bytes[i] == b'\'' {
                    // Check for escaped quote ''
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    } else {
                        in_single_quote = false;
                    }
                }
                i += 1;
                continue;
            }

            // End block comment
            if in_block_comment {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    in_block_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            // End line comment on newline
            if in_line_comment {
                if bytes[i] == b'\n' {
                    in_line_comment = false;
                }
                i += 1;
                continue;
            }

            // Detect start of block comment
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                in_block_comment = true;
                i += 2;
                continue;
            }

            // Detect start of line comment
            if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                in_line_comment = true;
                i += 2;
                continue;
            }

            // Detect start of single-quoted string
            if bytes[i] == b'\'' {
                in_single_quote = true;
                i += 1;
                continue;
            }

            // Detect || operator
            if i + 1 < len && bytes[i] == b'|' && bytes[i + 1] == b'|' {
                let has_space_before = i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
                let after_idx = i + 2;
                let has_space_after = after_idx >= len
                    || bytes[after_idx] == b' '
                    || bytes[after_idx] == b'\n'
                    || bytes[after_idx] == b'\r';

                if !has_space_before || !has_space_after {
                    let (line, col) = byte_offset_to_line_col(src, i);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "Missing space around || concat operator \u{2014} use 'a || b' style"
                            .to_string(),
                        line,
                        col,
                    });
                }

                // Skip past the ||
                i += 2;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Convert a byte offset into 1-based (line, col).
fn byte_offset_to_line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
