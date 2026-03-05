use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TrailingNewline;

impl Rule for TrailingNewline {
    fn name(&self) -> &'static str {
        "Layout/TrailingNewline"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Empty file — no violation
        if ctx.source.is_empty() {
            return Vec::new();
        }
        // File already ends with a newline — no violation
        if ctx.source.ends_with('\n') {
            return Vec::new();
        }
        // Missing trailing newline — emit one diagnostic
        let line_count = ctx.source.lines().count();
        let last_line_len = ctx.source.lines().last().map(|l| l.len()).unwrap_or(0);
        vec![Diagnostic {
            rule: self.name(),
            message: "File must end with a newline".to_string(),
            line: line_count,
            col: last_line_len + 1,
        }]
    }
}
