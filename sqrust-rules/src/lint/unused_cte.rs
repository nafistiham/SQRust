use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;

pub struct UnusedCte;

impl Rule for UnusedCte {
    fn name(&self) -> &'static str {
        "Lint/UnusedCte"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                if let Some(with) = &query.with {
                    for cte in &with.cte_tables {
                        let name = &cte.alias.name.value;
                        let name_upper = name.to_uppercase();

                        // Find the first occurrence of the name (the definition in WITH).
                        if let Some(first_pos) = source_upper.find(&name_upper) {
                            // Search the text that follows the definition for a second reference.
                            let after_def = &source_upper[first_pos + name_upper.len()..];
                            let appears_again = contains_word(after_def, &name_upper);

                            if !appears_again {
                                let (line, col) = offset_to_line_col(source, first_pos);
                                diags.push(Diagnostic {
                                    rule: self.name(),
                                    message: format!(
                                        "CTE '{}' is defined but never used",
                                        name
                                    ),
                                    line,
                                    col,
                                });
                            }
                        }
                    }
                }
            }
        }

        diags
    }
}

/// Returns true if `text` contains `word` as a whole word (alphanumeric / underscore
/// word boundaries on both sides, case-sensitive — callers normalise to uppercase).
fn contains_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let word_bytes = word.as_bytes();
    let word_len = word_bytes.len();
    let text_len = bytes.len();

    let mut pos = 0usize;
    while pos + word_len <= text_len {
        // Find the next candidate position using a simple scan.
        // (The slice passed in is already upper-cased, so direct byte comparison works.)
        if let Some(rel) = text[pos..].find(word) {
            let abs = pos + rel;

            let before_ok = abs == 0
                || {
                    let b = bytes[abs - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
            let after = abs + word_len;
            let after_ok = after >= text_len
                || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

            if before_ok && after_ok {
                return true;
            }
            pos = abs + 1;
        } else {
            break;
        }
    }

    false
}

/// Converts a byte offset into `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
