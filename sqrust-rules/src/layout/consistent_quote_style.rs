use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ConsistentQuoteStyle;

impl Rule for ConsistentQuoteStyle {
    fn name(&self) -> &'static str {
        "Layout/ConsistentQuoteStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let src = &ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();

        let mut in_single_quote = false;
        let mut in_block_comment = false;
        let mut in_line_comment = false;

        let mut single_quote_count: usize = 0;
        let mut double_quote_count: usize = 0;
        // Track position (byte offset) of the first double-quoted string
        let mut first_double_quote_pos: Option<usize> = None;

        let mut i = 0;
        while i < len {
            // Handle escape sequence: '' inside a single-quoted string is an escaped quote
            if in_single_quote {
                if bytes[i] == b'\'' {
                    // Check if next char is also a quote (escaped quote)
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        // Skip the escaped quote pair
                        i += 2;
                        continue;
                    } else {
                        // End of single-quoted string
                        in_single_quote = false;
                        i += 1;
                        continue;
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
                single_quote_count += 1;
                i += 1;
                continue;
            }

            // Detect start of double-quoted string
            if bytes[i] == b'"' {
                if first_double_quote_pos.is_none() {
                    first_double_quote_pos = Some(i);
                }
                double_quote_count += 1;
                // Scan to end of double-quoted string
                i += 1;
                while i < len && bytes[i] != b'"' {
                    i += 1;
                }
                // Skip the closing quote
                if i < len {
                    i += 1;
                }
                continue;
            }

            i += 1;
        }

        if single_quote_count > 0 && double_quote_count > 0 {
            // Compute line/col from the position of the first double-quoted string
            let pos = first_double_quote_pos.unwrap_or(0);
            let (line, col) = byte_offset_to_line_col(src, pos);
            vec![Diagnostic {
                rule: self.name(),
                message:
                    "Mixed string quote styles detected \u{2014} use single quotes consistently for string literals"
                        .to_string(),
                line,
                col,
            }]
        } else {
            Vec::new()
        }
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
