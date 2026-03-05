use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct StatementSemicolons;

impl Rule for StatementSemicolons {
    fn name(&self) -> &'static str {
        "Layout/StatementSemicolons"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // If there are parse errors we cannot reliably determine statement
        // boundaries, so skip.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;

        // Empty or whitespace-only sources have no statements to check.
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        // Check if the last non-whitespace character is a semicolon.
        let last_char = trimmed.chars().next_back();
        if last_char == Some(';') {
            return Vec::new();
        }

        // No trailing semicolon — find the last non-empty line.
        let (last_line_no, _) = last_non_empty_line(source);

        vec![Diagnostic {
            rule: self.name(),
            message: "SQL statement is missing a trailing semicolon".to_string(),
            line: last_line_no,
            col: 1,
        }]
    }
}

/// Returns the 1-indexed line number and content of the last non-empty
/// (non-whitespace-only) line in `source`.
/// Falls back to (1, "") if the source has no non-empty lines.
fn last_non_empty_line(source: &str) -> (usize, &str) {
    let mut last = (1usize, "");
    for (i, line) in source.lines().enumerate() {
        if !line.trim().is_empty() {
            last = (i + 1, line);
        }
    }
    last
}
