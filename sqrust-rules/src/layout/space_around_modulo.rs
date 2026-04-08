use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct SpaceAroundModulo;

impl Rule for SpaceAroundModulo {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundModulo"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if source.is_empty() {
        return Vec::new();
    }

    let skip = SkipMap::build(source);
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    for i in 0..len {
        if bytes[i] != b'%' {
            continue;
        }

        if !skip.is_code(i) {
            continue;
        }

        let space_before = i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t';
        let space_after = i + 1 >= len || bytes[i + 1] == b' ' || bytes[i + 1] == b'\t';

        if !space_before || !space_after {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Missing space around modulo operator '%'".to_string(),
                line,
                col,
            });
        }
    }

    diags
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p).unwrap_or(offset + 1);
    (line, col)
}
