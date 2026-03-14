use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MaxLineCount {
    pub max_lines: usize,
}

impl Default for MaxLineCount {
    fn default() -> Self {
        Self { max_lines: 500 }
    }
}

impl Rule for MaxLineCount {
    fn name(&self) -> &'static str {
        "Layout/MaxLineCount"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if ctx.source.is_empty() {
            return Vec::new();
        }

        let line_count = ctx.source.bytes().filter(|&b| b == b'\n').count() + 1;

        // A file that ends with a newline (e.g. "SELECT 1\n") has one trailing
        // empty line that we do not want to count as a real line.
        let effective = if ctx.source.ends_with('\n') {
            line_count - 1
        } else {
            line_count
        };

        if effective > self.max_lines {
            vec![Diagnostic {
                rule: self.name(),
                message: format!(
                    "File has {} lines which exceeds the maximum of {} — consider breaking into smaller files or CTEs",
                    effective, self.max_lines
                ),
                line: 1,
                col: 1,
            }]
        } else {
            Vec::new()
        }
    }
}
