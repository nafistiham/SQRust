use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TabIndentation;

impl Rule for TabIndentation {
    fn name(&self) -> &'static str {
        "Layout/TabIndentation"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for (line_num, line) in ctx.lines() {
            // Flag only lines whose first character is a tab (leading tab)
            if line.starts_with('\t') {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Avoid tab characters for indentation; use spaces".to_string(),
                    line: line_num,
                    col: 1,
                });
            }
        }
        diags
    }
}
