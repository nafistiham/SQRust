use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MultipleStatementsInFile;

impl Rule for MultipleStatementsInFile {
    fn name(&self) -> &'static str {
        "Structure/MultipleStatementsInFile"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let n = ctx.statements.len();
        if n > 1 {
            vec![Diagnostic {
                rule: self.name(),
                message: format!(
                    "File contains {} statements; consider splitting into separate files \
                     for clearer CI/CD execution and dbt model isolation",
                    n
                ),
                line: 1,
                col: 1,
            }]
        } else {
            Vec::new()
        }
    }
}
