use sqrust_core::{Diagnostic, FileContext, Rule};

use super::{is_word_char, SkipMap};

/// SQL data type names that must be written in UPPERCASE.
/// Stored as uppercase for comparison purposes.
/// Sorted by length descending so that longer names (e.g. SMALLINT) are
/// checked before shorter prefixes (e.g. INT), preventing partial matches.
const TYPES: &[&str] = &[
    "CURRENT_TIMESTAMP",
    "VARBINARY",
    "TIMESTAMP",
    "NVARCHAR",
    "SMALLINT",
    "DATETIME",
    "INTERVAL",
    "NUMERIC",
    "BOOLEAN",
    "INTEGER",
    "DECIMAL",
    "TINYINT",
    "VARCHAR",
    "DOUBLE",
    "BIGINT",
    "BINARY",
    "NCHAR",
    "FLOAT",
    "CLOB",
    "TEXT",
    "BOOL",
    "REAL",
    "DATE",
    "BLOB",
    "UUID",
    "JSONB",
    "TIME",
    "JSON",
    "CHAR",
    "ARRAY",
    "NUMBER",
    "BYTEA",
    "INT",
    "BIT",
];

pub struct Types;

impl Rule for Types {
    fn name(&self) -> &'static str {
        "Capitalisation/Types"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        let mut i = 0;
        while i < len {
            // Only enter type detection on a word-start in code that is not
            // preceded by a word character.
            if skip_map.is_code(i) && is_word_char(bytes[i]) {
                let preceded_by_word = i > 0 && is_word_char(bytes[i - 1]);
                if !preceded_by_word {
                    // Find end of this word token.
                    let word_start = i;
                    let mut j = i;
                    while j < len && is_word_char(bytes[j]) {
                        j += 1;
                    }
                    let word_end = j; // exclusive

                    // The whole word must be in code (not inside a string/comment).
                    let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));

                    if all_code {
                        let word_bytes = &bytes[word_start..word_end];

                        // Try each type name (already sorted longest-first).
                        for type_name in TYPES {
                            if type_name.len() == word_bytes.len()
                                && type_name
                                    .bytes()
                                    .zip(word_bytes.iter())
                                    .all(|(a, &b)| a.eq_ignore_ascii_case(&b))
                            {
                                // Matched — is it already all-uppercase?
                                let already_upper = word_bytes
                                    .iter()
                                    .all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic());
                                if !already_upper {
                                    let (line, col) = line_col(source, word_start);
                                    let found =
                                        std::str::from_utf8(word_bytes).unwrap_or("?").to_string();
                                    let upper = *type_name;
                                    diags.push(Diagnostic {
                                        rule: self.name(),
                                        message: format!(
                                            "Data type '{}' should be '{}'",
                                            found, upper
                                        ),
                                        line,
                                        col,
                                    });
                                }
                                // Whether or not it was a violation, stop checking
                                // this word against further (shorter) type names.
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
