use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LeadingComma;

impl Rule for LeadingComma {
    fn name(&self) -> &'static str {
        "Layout/LeadingComma"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        // Text-based scan: works even when ctx.parse_errors is non-empty.
        // Split on '\n' (not .lines()) so we preserve accurate line indices.
        let lines: Vec<&str> = ctx.source.split('\n').collect();
        // Track whether we are inside a single-quoted string literal across lines.
        // Uses a simple odd/even quote-count heuristic: each unescaped `'` toggles
        // the in_string state. Good enough for the rare "comma-at-line-start inside
        // a multi-line string" false-positive prevention.
        let mut in_string = false;

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();

            if !in_string && trimmed.starts_with(',') {
                // col is 1-indexed position of ',' in the original line
                let leading_spaces = line.len() - trimmed.len();
                let col = leading_spaces + 1;
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Comma at start of line; place commas at the end of the previous line"
                        .to_string(),
                    line: idx + 1,
                    col,
                });
            }

            // Update in_string: each single-quote character toggles the state.
            // Odd number of quotes on the line means we cross a string boundary.
            let quote_count = line.chars().filter(|&c| c == '\'').count();
            if quote_count % 2 != 0 {
                in_string = !in_string;
            }
        }

        diags
    }
}
