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

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                if let Some(with) = &query.with {
                    for cte in &with.cte_tables {
                        let name = &cte.alias.name.value;

                        // Find the first occurrence of the name using ASCII case-insensitive
                        // comparison on the original source — avoids byte-offset mismatch
                        // that can occur when to_uppercase() changes multi-byte sequences.
                        if let Some(first_pos) = find_word_ascii_ci(source, name) {
                            // Search the text that follows the definition for a second reference.
                            let after_def = &source[first_pos + name.len()..];
                            let appears_again = find_word_ascii_ci(after_def, name).is_some();

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

/// Finds the first occurrence of `word` as a whole word in `text` using ASCII
/// case-insensitive comparison. Operates on the original bytes so byte offsets
/// remain valid for the original string even when `text` contains non-ASCII chars.
fn find_word_ascii_ci(text: &str, word: &str) -> Option<usize> {
    let src = text.as_bytes();
    let wrd: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = wrd.len();
    if wlen == 0 {
        return None;
    }
    let mut pos = 0usize;
    while pos + wlen <= src.len() {
        if src[pos..pos + wlen]
            .iter()
            .zip(wrd.iter())
            .all(|(a, b)| a.to_ascii_uppercase() == *b)
        {
            let before_ok = pos == 0 || {
                let b = src[pos - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = pos + wlen;
            let after_ok = after >= src.len() || {
                let b = src[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok && after_ok {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
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
