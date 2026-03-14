use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct OperatorAtLineStart;

impl Rule for OperatorAtLineStart {
    fn name(&self) -> &'static str {
        "Layout/OperatorAtLineStart"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for (line_idx, line) in source.split('\n').enumerate() {
        let line_num = line_idx + 1;

        // Strip an inline line comment (-- ...) before checking for trailing keyword.
        // Only strip if the `--` is not inside a string.
        let code_part = strip_line_comment(line);

        // Trim trailing whitespace for the word-boundary check.
        let trimmed = code_part.trim_end();

        if trimmed.is_empty() {
            continue;
        }

        let upper = trimmed.to_uppercase();

        // Check for trailing AND (word-bounded: preceded by a space or tab).
        if upper.ends_with(" AND") || upper.ends_with("\tAND") {
            // Ensure the AND is a standalone word — not something like "GRAND".
            let keyword_start = trimmed.len() - 3; // len of "AND"
            let col = keyword_start + 1; // 1-indexed
            diags.push(Diagnostic {
                rule: rule_name,
                message: "AND at end of line; prefer leading operators \u{2014} move AND to the start of the next line".to_string(),
                line: line_num,
                col,
            });
            continue;
        }

        // Check for trailing OR (word-bounded: preceded by a space or tab).
        // Guard against "ORDER", "COLOR", etc. — OR must be preceded by whitespace.
        if upper.ends_with(" OR") || upper.ends_with("\tOR") {
            let keyword_start = trimmed.len() - 2; // len of "OR"
            let col = keyword_start + 1; // 1-indexed
            diags.push(Diagnostic {
                rule: rule_name,
                message: "OR at end of line; prefer leading operators \u{2014} move OR to the start of the next line".to_string(),
                line: line_num,
                col,
            });
        }
    }

    diags
}

/// Strips a line comment (`-- ...`) from the end of a line, respecting single-quoted strings.
/// Returns the code portion before the comment.
fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut in_string = false;
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if in_string {
            if b == b'\'' {
                // Handle '' escaped quote.
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
        } else if b == b'\'' {
            in_string = true;
        } else if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
            // Found a line comment outside a string.
            return &line[..i];
        }
        i += 1;
    }

    line
}
