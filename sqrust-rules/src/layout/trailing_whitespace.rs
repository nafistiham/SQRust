use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TrailingWhitespace;

impl Rule for TrailingWhitespace {
    fn name(&self) -> &'static str {
        "Layout/TrailingWhitespace"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for (line_num, line) in ctx.lines() {
            let trimmed = line.trim_end();
            if trimmed.len() < line.len() {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Trailing whitespace".to_string(),
                    line: line_num,
                    col: trimmed.len() + 1,
                });
            }
        }
        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let lines: Vec<&str> = ctx.source.lines().map(|l| l.trim_end()).collect();
        let mut fixed = lines.join("\n");
        if ctx.source.ends_with('\n') {
            fixed.push('\n');
        }
        Some(fixed)
    }
}
