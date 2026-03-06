use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NestedParentheses {
    pub max_depth: usize,
}

impl Default for NestedParentheses {
    fn default() -> Self {
        NestedParentheses { max_depth: 5 }
    }
}

impl Rule for NestedParentheses {
    fn name(&self) -> &'static str {
        "Layout/NestedParentheses"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name(), self.max_depth)
    }
}

fn find_violations(source: &str, rule_name: &'static str, max_depth: usize) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut depth = 0usize;
    let mut in_string = false;
    // True while we are still at or above the first excess depth level for the
    // current "nested group". Reset to false when depth drops back to max_depth.
    let mut over_max = false;

    let mut i = 0usize;
    while i < len {
        let byte = bytes[i];

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

        if byte == b'(' {
            depth += 1;
            if depth > max_depth && !over_max {
                over_max = true;
                let (line, col) = byte_offset_to_line_col(source, i);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: format!(
                        "Parenthesis nesting depth {} exceeds maximum of {}",
                        depth, max_depth
                    ),
                    line,
                    col,
                });
            }
        } else if byte == b')' {
            if depth > 0 {
                depth -= 1;
            }
            // Once we drop back to max_depth or below, reset the flag so the
            // next time we go over we flag again.
            if depth <= max_depth {
                over_max = false;
            }
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
