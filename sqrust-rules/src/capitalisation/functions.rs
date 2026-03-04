use sqrust_core::{Diagnostic, FileContext, Rule};

use super::{is_word_char, SkipMap};

/// SQL built-in function names that must be written in UPPERCASE when used
/// as function calls (i.e. immediately followed by `(`).
const FUNCTIONS: &[&str] = &[
    "COUNT",
    "SUM",
    "MAX",
    "MIN",
    "AVG",
    "COALESCE",
    "NULLIF",
    "CAST",
    "CONVERT",
    "UPPER",
    "LOWER",
    "LENGTH",
    "TRIM",
    "LTRIM",
    "RTRIM",
    "SUBSTR",
    "SUBSTRING",
    "REPLACE",
    "CONCAT",
    "NOW",
    "CURRENT_DATE",
    "CURRENT_TIMESTAMP",
    "DATE_TRUNC",
    "EXTRACT",
    "ROUND",
    "FLOOR",
    "CEIL",
    "ABS",
    "MOD",
    "POWER",
    "SQRT",
    "ROW_NUMBER",
    "RANK",
    "DENSE_RANK",
    "LAG",
    "LEAD",
    "FIRST_VALUE",
    "LAST_VALUE",
    "NTH_VALUE",
    "NTILE",
    "PERCENT_RANK",
    "CUME_DIST",
    "ARRAY_AGG",
    "STRING_AGG",
    "BOOL_AND",
    "BOOL_OR",
    "VARIANCE",
    "STDDEV",
    "UNNEST",
    "GREATEST",
    "LEAST",
];

pub struct Functions;

impl Rule for Functions {
    fn name(&self) -> &'static str {
        "Capitalisation/Functions"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        let mut i = 0;
        while i < len {
            // Enter token detection on a word-start in code that is not
            // preceded by a word character.
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

                    // The whole word must be in code
                    let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));

                    if all_code {
                        // A function call requires an immediate `(` right after the word
                        // (word_end must point to `(` in code).
                        let followed_by_paren = word_end < len
                            && bytes[word_end] == b'('
                            && skip_map.is_code(word_end);

                        if followed_by_paren {
                            let word_bytes = &bytes[word_start..word_end];

                            for func in FUNCTIONS {
                                if func.len() == word_bytes.len()
                                    && func
                                        .bytes()
                                        .zip(word_bytes.iter())
                                        .all(|(a, &b)| a.eq_ignore_ascii_case(&b))
                                {
                                    // Matched — is it already uppercase?
                                    let already_upper = word_bytes.iter().all(|b| {
                                        b.is_ascii_uppercase() || !b.is_ascii_alphabetic()
                                    });
                                    if !already_upper {
                                        let (line, col) = line_col(source, word_start);
                                        let found = std::str::from_utf8(word_bytes)
                                            .unwrap_or("?")
                                            .to_string();
                                        let upper = found.to_uppercase();
                                        diags.push(Diagnostic {
                                            rule: self.name(),
                                            message: format!(
                                                "Function '{}' should be UPPERCASE (use '{}')",
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
