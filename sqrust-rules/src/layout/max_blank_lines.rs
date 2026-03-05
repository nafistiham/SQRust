use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MaxBlankLines {
    pub max_blank_lines: usize,
}

impl Default for MaxBlankLines {
    fn default() -> Self {
        Self { max_blank_lines: 1 }
    }
}

impl Rule for MaxBlankLines {
    fn name(&self) -> &'static str {
        "MaxBlankLines"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let lines: Vec<&str> = ctx.source.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            if lines[i].trim().is_empty() {
                // Found the start of a blank-line run.
                let run_start = i; // 0-indexed position of first blank line in run
                let mut run_len = 0usize;
                while i < lines.len() && lines[i].trim().is_empty() {
                    run_len += 1;
                    i += 1;
                }
                // Flag if the run exceeds the maximum.
                if run_len > self.max_blank_lines {
                    // Violation line: the (max_blank_lines + 1)-th blank in the run (1-indexed).
                    let violation_line = run_start + self.max_blank_lines + 1; // 1-indexed
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Too many consecutive blank lines ({} found, maximum is {})",
                            run_len,
                            self.max_blank_lines
                        ),
                        line: violation_line,
                        col: 1,
                    });
                }
            } else {
                i += 1;
            }
        }

        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let violations = self.check(ctx);
        if violations.is_empty() {
            return None;
        }

        let lines: Vec<&str> = ctx.source.lines().collect();
        let mut result: Vec<&str> = Vec::with_capacity(lines.len());
        let mut blank_run = 0usize;

        for line in &lines {
            if line.trim().is_empty() {
                blank_run += 1;
                if blank_run <= self.max_blank_lines {
                    result.push(line);
                }
                // Lines beyond the max are silently dropped.
            } else {
                blank_run = 0;
                result.push(line);
            }
        }

        let mut fixed = result.join("\n");
        if ctx.source.ends_with('\n') {
            fixed.push('\n');
        }
        Some(fixed)
    }
}
