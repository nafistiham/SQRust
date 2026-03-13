use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct IndentationConsistency;

impl Rule for IndentationConsistency {
    fn name(&self) -> &'static str {
        "Layout/IndentationConsistency"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let counts = collect_leading_space_counts(&ctx.source);

        if counts.is_empty() {
            return Vec::new();
        }

        let g = counts.iter().copied().fold(0usize, gcd);

        if g <= 1 {
            return vec![Diagnostic {
                rule: self.name(),
                message: "Inconsistent indentation detected — lines use mixed indentation widths"
                    .to_string(),
                line: 1,
                col: 1,
            }];
        }

        Vec::new()
    }
}

/// Collect the leading-space counts (> 0) for every non-empty, non-comment line.
fn collect_leading_space_counts(source: &str) -> Vec<usize> {
    let mut result = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim_start_matches(' ');

        // Skip blank lines.
        if trimmed.is_empty() {
            continue;
        }

        // Skip lines that consist entirely of leading spaces then a comment marker.
        if trimmed.starts_with("--") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        let leading = line.len() - trimmed.len();
        if leading > 0 {
            result.push(leading);
        }
    }

    result
}

/// Greatest common divisor (Euclidean algorithm).
fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
