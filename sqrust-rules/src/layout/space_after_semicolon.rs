use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SpaceAfterSemicolon;

impl Rule for SpaceAfterSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterSemicolon"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();

        // Track string literal state to skip semicolons inside strings.
        let mut in_string = false;
        let mut i = 0;

        while i < len {
            if !in_string && bytes[i] == b'\'' {
                in_string = true;
                i += 1;
                continue;
            }
            if in_string {
                if bytes[i] == b'\'' {
                    // Check for escaped ''
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                i += 1;
                continue;
            }

            if bytes[i] == b';' {
                // Find what follows the semicolon (skip whitespace)
                let semi_offset = i;
                let mut j = i + 1;
                // Skip spaces/tabs (but not newlines)
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                // If we hit a newline, EOF, or a comment — that's fine
                if j < len && bytes[j] != b'\n' && bytes[j] != b'\r' {
                    // Also allow if it's a comment start (--)
                    let is_comment = j + 1 < len && bytes[j] == b'-' && bytes[j + 1] == b'-';
                    let is_block_comment = j + 1 < len && bytes[j] == b'/' && bytes[j + 1] == b'*';
                    if !is_comment && !is_block_comment {
                        let (line, col) = offset_to_line_col(&ctx.source, semi_offset);
                        diags.push(Diagnostic {
                            rule: "Layout/SpaceAfterSemicolon",
                            message: "Semicolon must be followed by a newline or end of file; each statement should be on its own line".to_string(),
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
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
