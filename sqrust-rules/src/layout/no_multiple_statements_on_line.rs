use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoMultipleStatementsOnLine;

impl Rule for NoMultipleStatementsOnLine {
    fn name(&self) -> &'static str {
        "Layout/NoMultipleStatementsOnLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut i = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut block_depth: usize = 0;

    while i < len {
        // Reset line comment at newline
        if bytes[i] == b'\n' {
            in_line_comment = false;
            i += 1;
            continue;
        }

        // If inside a line comment, skip everything until newline
        if in_line_comment {
            i += 1;
            continue;
        }

        // Single-quoted string handling (outside block comment)
        if !in_string && block_depth == 0 && bytes[i] == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\'' {
                // SQL '' escape
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Block comment open: /*
        if block_depth == 0 && i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_depth += 1;
            i += 2;
            continue;
        }
        // Inside block comment
        if block_depth > 0 {
            if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // Line comment start: --
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            in_line_comment = true;
            i += 2;
            continue;
        }

        // Semicolon found outside strings and comments
        if bytes[i] == b';' {
            let mut j = i + 1;

            // Skip spaces and tabs (not newlines)
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }

            // EOF or newline → no violation
            if j >= len || bytes[j] == b'\n' || bytes[j] == b'\r' {
                i += 1;
                continue;
            }

            // If next non-whitespace starts a line comment '--' → no violation
            if j + 1 < len && bytes[j] == b'-' && bytes[j + 1] == b'-' {
                i += 1;
                continue;
            }

            // Otherwise: there is real content after the semicolon on the same line
            let (line, col) = byte_offset_to_line_col(source, j);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Multiple statements on the same line; each statement should be on its own line".to_string(),
                line,
                col,
            });
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
