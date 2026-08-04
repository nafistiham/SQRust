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
        "Layout/MaxBlankLines"
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

        // Copy each line together with its original terminator. Using
        // `lines()` + `join("\n")` here would rewrite a CRLF file as LF as a
        // side effect of dropping blank lines.
        let source = &ctx.source;
        let mut fixed = String::with_capacity(source.len());
        let mut rest = source.as_str();
        let mut blank_run = 0usize;

        loop {
            match rest.find('\n') {
                Some(nl) => {
                    let (chunk, tail) = rest.split_at(nl + 1);
                    let without_nl = &chunk[..chunk.len() - 1];
                    let content = without_nl.strip_suffix('\r').unwrap_or(without_nl);

                    if content.trim().is_empty() {
                        blank_run += 1;
                        // Lines beyond the max are silently dropped.
                        if blank_run <= self.max_blank_lines {
                            fixed.push_str(chunk);
                        }
                    } else {
                        blank_run = 0;
                        fixed.push_str(chunk);
                    }
                    rest = tail;
                }
                None => {
                    // Final segment with no trailing newline.
                    if !rest.is_empty() {
                        if rest.trim().is_empty() {
                            blank_run += 1;
                            if blank_run <= self.max_blank_lines {
                                fixed.push_str(rest);
                            }
                        } else {
                            fixed.push_str(rest);
                        }
                    }
                    break;
                }
            }
        }

        if fixed == *source {
            None
        } else {
            Some(fixed)
        }
    }
}
