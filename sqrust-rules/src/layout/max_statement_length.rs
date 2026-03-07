use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MaxStatementLength {
    pub max_lines: usize,
}

impl Default for MaxStatementLength {
    fn default() -> Self {
        MaxStatementLength { max_lines: 50 }
    }
}

impl Rule for MaxStatementLength {
    fn name(&self) -> &'static str {
        "Layout/MaxStatementLength"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let statements = split_statements(&ctx.source);

        for (byte_offset, stmt_text) in statements {
            let line_count = count_lines(stmt_text);
            if line_count == 0 {
                // Empty / whitespace-only statement — skip
                continue;
            }

            if line_count > self.max_lines {
                // Determine the 1-indexed line number where this statement starts
                // in the full source by counting newlines before the statement.
                let start_line = ctx.source[..byte_offset]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count()
                    + 1;

                // Find the first non-whitespace character column (1-indexed) on
                // the starting line.
                let start_col = ctx
                    .source
                    .lines()
                    .nth(start_line - 1)
                    .map(|l| {
                        l.chars()
                            .position(|c| !c.is_whitespace())
                            .map(|p| p + 1)
                            .unwrap_or(1)
                    })
                    .unwrap_or(1);

                diags.push(Diagnostic {
                    rule: self.name(),
                    message: format!(
                        "Statement spans {} lines, exceeding the maximum of {} lines",
                        line_count, self.max_lines
                    ),
                    line: start_line,
                    col: start_col,
                });
            }
        }

        diags
    }
}

/// Count lines spanned by the statement text (including trailing `;` line).
/// Returns 0 for whitespace-only text so empty statements are skipped.
fn count_lines(text: &str) -> usize {
    if text.trim().is_empty() {
        return 0;
    }
    // Trim only *leading* blank lines (a statement may have trailing content
    // on the `;` line).  We keep the trailing `;` line in the count.
    // `lines()` splits on `\n` and includes partial lines at the end.
    text.lines().count()
}

/// Split `source` into (byte_offset_of_start, statement_text) pairs.
///
/// Each returned slice extends *through* the `;` terminator (inclusive) so
/// that the `;` line is counted as part of the statement span.  Splitting
/// respects single-quoted string literals so embedded `;` are not treated as
/// terminators.
///
/// The trailing remainder (no `;`) is also returned if non-empty.
fn split_statements(source: &str) -> Vec<(usize, &str)> {
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];

        if in_string {
            if b == b'\'' {
                // Doubled-quote escape: ''
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
        } else {
            match b {
                b'\'' => {
                    in_string = true;
                }
                b';' => {
                    // Include the `;` in the statement text (end is i+1).
                    statements.push((start, &source[start..=i]));
                    start = i + 1;
                }
                _ => {}
            }
        }

        i += 1;
    }

    // Remainder after last `;` (or entire source if no `;` present)
    if start < source.len() {
        let remainder = &source[start..];
        if !remainder.trim().is_empty() {
            statements.push((start, remainder));
        }
    }

    statements
}
