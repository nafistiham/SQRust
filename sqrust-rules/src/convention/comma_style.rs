use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CommaStyle;

impl Rule for CommaStyle {
    fn name(&self) -> &'static str {
        "Convention/CommaStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut has_trailing = false;
        let mut first_leading_line: Option<usize> = None;

        for (line_num, line) in ctx.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let last_non_ws = line.trim_end().chars().last();
            let first_non_ws = trimmed.chars().next();

            let is_trailing = last_non_ws == Some(',');
            let is_leading = first_non_ws == Some(',');

            if is_trailing {
                has_trailing = true;
            }
            if is_leading && first_leading_line.is_none() {
                first_leading_line = Some(line_num);
            }
        }

        if has_trailing {
            if let Some(leading_line) = first_leading_line {
                return vec![Diagnostic {
                    rule: self.name(),
                    message: "Inconsistent comma style: mix of leading and trailing commas"
                        .to_string(),
                    line: leading_line,
                    col: 1,
                }];
            }
        }

        vec![]
    }
}
