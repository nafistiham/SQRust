use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LeadingOperator;

impl Rule for LeadingOperator {
    fn name(&self) -> &'static str {
        "Layout/LeadingOperator"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        // Text-based scan: works even when ctx.parse_errors is non-empty.
        // Split on '\n' so we get accurate line indices.
        let lines: Vec<&str> = ctx.source.split('\n').collect();

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            let upper = trimmed.to_uppercase();

            // 1-indexed column of the operator keyword in the original line
            let leading_spaces = line.len() - trimmed.len();
            let col = leading_spaces + 1;

            // Check for AND at line start.
            // Word boundary: AND must be followed by whitespace or be the whole token
            // (i.e. end of line). No SQL keyword starts with "AND" that would cause
            // a false positive, so no additional guard needed.
            let is_and = upper == "AND"
                || upper.starts_with("AND ")
                || upper.starts_with("AND\t")
                || upper.starts_with("AND\r");

            if is_and {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message:
                        "AND at start of line; place operators at the end of the previous line"
                            .to_string(),
                    line: idx + 1,
                    col,
                });
                // A line can't be both AND and OR at the same time, so skip OR check.
                continue;
            }

            // Check for OR at line start (word boundary), but NOT ORDER BY.
            // "ORDER" starts with "OR" so we guard against it.
            let is_or_token = upper == "OR"
                || upper.starts_with("OR ")
                || upper.starts_with("OR\t")
                || upper.starts_with("OR\r");

            if is_or_token && !upper.starts_with("ORDER") {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message:
                        "OR at start of line; place operators at the end of the previous line"
                            .to_string(),
                    line: idx + 1,
                    col,
                });
            }
        }

        diags
    }
}
