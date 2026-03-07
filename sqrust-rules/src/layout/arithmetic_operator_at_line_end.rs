use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ArithmeticOperatorAtLineEnd;

impl Rule for ArithmeticOperatorAtLineEnd {
    fn name(&self) -> &'static str {
        "Layout/ArithmeticOperatorAtLineEnd"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        for (line_num, line) in ctx.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }

            // Skip whole-line comments (-- ...)
            let stripped = trimmed.trim_start();
            if stripped.starts_with("--") {
                continue;
            }

            // Get the last character of the trimmed line
            let last_char = match trimmed.chars().last() {
                Some(c) => c,
                None => continue,
            };

            // Determine whether the trailing character is a flaggable operator.
            // Rules:
            //   '+' => flag
            //   '/' => flag
            //   '-' => flag only if preceded by a non-'-' character (guards against trailing --)
            //   '*' => exempt (SELECT *, COUNT(*))
            let op_char: Option<char> = match last_char {
                '+' | '/' => Some(last_char),
                '-' => {
                    // Look at the second-to-last char to rule out "--"
                    let second_last = trimmed.chars().rev().nth(1);
                    if second_last == Some('-') {
                        None
                    } else {
                        Some('-')
                    }
                }
                _ => None,
            };

            let op_char = match op_char {
                Some(c) => c,
                None => continue,
            };

            // Check whether the operator character is inside a single-quoted string.
            // We do a simple single-pass scan of the trimmed line tracking string state.
            // This handles single-line strings correctly.  Cross-line strings are uncommon
            // in SQL and are treated as outside-string for the purpose of this check.
            if operator_is_in_string(trimmed) {
                continue;
            }

            // col is 1-indexed position of the trailing operator character.
            // trimmed has the same leading content as line, just trailing whitespace stripped,
            // so trimmed.len() characters are already the exact byte-length up to (and
            // including) the operator.
            let col = trimmed.len();

            diags.push(Diagnostic {
                rule: self.name(),
                message: format!(
                    "Arithmetic operator '{}' at line end; move to start of next line for clarity",
                    op_char
                ),
                line: line_num,
                col,
            });
        }

        diags
    }
}

/// Returns true when the last character of `line` is inside a single-quoted
/// string literal.  Handles escaped-quote via doubled-quote (`''`) convention.
fn operator_is_in_string(line: &str) -> bool {
    let mut in_string = false;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if in_string {
            if c == '\'' {
                // Peek ahead: '' is an escaped quote, stay in string
                if i + 1 < len && chars[i + 1] == '\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
        } else if c == '\'' {
            in_string = true;
        }
        i += 1;
    }

    // If we finished the scan still inside a string, the last character is in
    // a string (the string didn't close on this line).
    in_string
}
