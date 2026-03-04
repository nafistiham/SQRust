use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct Coalesce;

/// Non-ANSI null-coalescing function names to flag (ordered longest-first to avoid
/// matching "NVL" when "NVL2" is present at the same position).
const NON_ANSI_FUNCS: &[&str] = &["ISNULL", "IFNULL", "NVL2", "NVL"];

impl Rule for Coalesce {
    fn name(&self) -> &'static str {
        "Convention/Coalesce"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut diags = Vec::new();

        let mut i = 0;
        let mut line = 1usize;
        let mut col = 1usize;

        while i < len {
            let ch = chars[i];

            // Line comment: skip to end of line
            if ch == '-' && i + 1 < len && chars[i + 1] == '-' {
                while i < len && chars[i] != '\n' {
                    col += 1;
                    i += 1;
                }
                continue;
            }

            // Block comment: /* ... */
            if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
                col += 2;
                i += 2;
                while i < len {
                    if chars[i] == '\n' {
                        line += 1;
                        col = 1;
                        i += 1;
                    } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                        col += 2;
                        i += 2;
                        break;
                    } else {
                        col += 1;
                        i += 1;
                    }
                }
                continue;
            }

            // Single-quoted string: skip ('' is the escape sequence for a literal quote)
            if ch == '\'' {
                col += 1;
                i += 1;
                while i < len {
                    if chars[i] == '\'' {
                        if i + 1 < len && chars[i + 1] == '\'' {
                            col += 2;
                            i += 2;
                        } else {
                            col += 1;
                            i += 1;
                            break;
                        }
                    } else if chars[i] == '\n' {
                        line += 1;
                        col = 1;
                        i += 1;
                    } else {
                        col += 1;
                        i += 1;
                    }
                }
                continue;
            }

            // Double-quoted identifier: skip
            if ch == '"' {
                col += 1;
                i += 1;
                while i < len {
                    if chars[i] == '"' {
                        col += 1;
                        i += 1;
                        break;
                    } else if chars[i] == '\n' {
                        line += 1;
                        col = 1;
                        i += 1;
                    } else {
                        col += 1;
                        i += 1;
                    }
                }
                continue;
            }

            if ch == '\n' {
                line += 1;
                col = 1;
                i += 1;
                continue;
            }

            // Try to match a non-ANSI function name followed immediately by '('
            let matched_func = NON_ANSI_FUNCS.iter().find(|&&func| {
                let func_len = func.len();
                if i + func_len >= len {
                    return false;
                }
                if chars[i + func_len] != '(' {
                    return false;
                }
                let candidate: String = chars[i..i + func_len]
                    .iter()
                    .map(|c| c.to_ascii_uppercase())
                    .collect();
                candidate == *func
            });

            if let Some(&func_name) = matched_func {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: format!(
                        "Use COALESCE instead of {}()",
                        func_name.to_uppercase()
                    ),
                    line,
                    col,
                });
                col += func_name.len();
                i += func_name.len();
                continue;
            }

            col += 1;
            i += 1;
        }

        diags
    }
}
