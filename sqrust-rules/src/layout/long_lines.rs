use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LongLines {
    pub max_length: usize,
}

impl Default for LongLines {
    fn default() -> Self {
        LongLines { max_length: 120 }
    }
}

impl Rule for LongLines {
    fn name(&self) -> &'static str {
        "Layout/LongLines"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for (line_num, line) in ctx.lines() {
            let length = line.chars().count();
            if length > self.max_length {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: format!(
                        "Line is {} characters, maximum is {}",
                        length, self.max_length
                    ),
                    line: line_num,
                    col: self.max_length + 1,
                });
            }
        }
        diags
    }
}
