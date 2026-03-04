use sqrust_core::{Diagnostic, FileContext, Rule};

use super::{is_word_char, SkipMap};

/// All SQL reserved keywords that must be written in UPPERCASE.
/// Stored as uppercase for comparison purposes.
const KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "FULL", "CROSS", "ON",
    "AND", "OR", "NOT", "IN", "LIKE", "IS", "NULL", "AS", "BY", "HAVING", "UNION", "ALL",
    "DISTINCT", "LIMIT", "OFFSET", "WITH", "CASE", "WHEN", "THEN", "ELSE", "END", "GROUP",
    "ORDER", "ASC", "DESC", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "TABLE",
    "INDEX", "VIEW", "SET", "INTO", "VALUES", "EXISTS", "BETWEEN", "OVER", "PARTITION", "USING",
    "NATURAL", "LATERAL", "RECURSIVE", "RETURNING", "EXCEPT", "INTERSECT", "FILTER",
];

pub struct Keywords;

impl Rule for Keywords {
    fn name(&self) -> &'static str {
        "Capitalisation/Keywords"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        // Walk every byte; when we find the start of a word that is code, try
        // to match it against the keyword list.
        let mut i = 0;
        while i < len {
            // Only enter keyword detection on a word-start that is code and is
            // not preceded by a word character.
            if skip_map.is_code(i) && is_word_char(bytes[i]) {
                let preceded_by_word = i > 0 && is_word_char(bytes[i - 1]);
                if !preceded_by_word {
                    // Find end of this word token
                    let word_start = i;
                    let mut j = i;
                    while j < len && is_word_char(bytes[j]) {
                        j += 1;
                    }
                    let word_end = j; // exclusive

                    // The whole word must be in code (no skip bytes inside it)
                    let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));

                    if all_code {
                        let word_bytes = &bytes[word_start..word_end];

                        // Check against keyword list (case-insensitive)
                        for kw in KEYWORDS {
                            if kw.len() == word_bytes.len()
                                && kw
                                    .bytes()
                                    .zip(word_bytes.iter())
                                    .all(|(a, &b)| a.eq_ignore_ascii_case(&b))
                            {
                                // It matches a keyword — is it already uppercase?
                                let already_upper = word_bytes
                                    .iter()
                                    .all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic());
                                if !already_upper {
                                    // Compute line + col (1-indexed)
                                    let (line, col) = line_col(source, word_start);
                                    let found =
                                        std::str::from_utf8(word_bytes).unwrap_or("?").to_string();
                                    let upper = found.to_uppercase();
                                    diags.push(Diagnostic {
                                        rule: self.name(),
                                        message: format!(
                                            "Keyword '{}' should be UPPERCASE (use '{}')",
                                            found, upper
                                        ),
                                        line,
                                        col,
                                    });
                                }
                                break;
                            }
                        }
                    }

                    i = word_end;
                    continue;
                }
            }
            i += 1;
        }

        diags
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
