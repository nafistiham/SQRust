use sqrust_core::{Diagnostic, FileContext, Rule};

use super::{is_word_char, SkipMap};

/// SQL boolean/null literals that must be written in UPPERCASE (SQLFluff CP04).
///
/// Flags `true`, `false`, and `null` (in any capitalisation) that are not
/// already fully uppercase.  Occurrences inside string literals, comments, or
/// quoted identifiers are ignored.
pub struct Literals;

/// The three literals we care about, stored in their canonical uppercase form.
const LITERALS: &[&str] = &["TRUE", "FALSE", "NULL"];

impl Rule for Literals {
    fn name(&self) -> &'static str {
        "Capitalisation/Literals"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Only enter detection on a word-start in real code that is not
            // preceded by another word character (ensures left word boundary).
            if skip_map.is_code(i) && is_word_char(bytes[i]) {
                let preceded_by_word = i > 0 && is_word_char(bytes[i - 1]);
                if !preceded_by_word {
                    // Collect the full word token.
                    let word_start = i;
                    let mut j = i;
                    while j < len && is_word_char(bytes[j]) {
                        j += 1;
                    }
                    let word_end = j; // exclusive

                    // Every byte of the word must be in code.
                    let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));

                    if all_code {
                        let word_bytes = &bytes[word_start..word_end];

                        for &lit in LITERALS {
                            // Length check first (cheap) then case-insensitive compare.
                            if lit.len() == word_bytes.len()
                                && lit
                                    .bytes()
                                    .zip(word_bytes.iter())
                                    .all(|(a, &b)| a.eq_ignore_ascii_case(&b))
                            {
                                // Word matches one of our literals — is it already uppercase?
                                let already_upper = word_bytes
                                    .iter()
                                    .all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic());

                                if !already_upper {
                                    let (line, col) = line_col(source, word_start);
                                    let found = std::str::from_utf8(word_bytes)
                                        .unwrap_or("?")
                                        .to_string();
                                    diags.push(Diagnostic {
                                        rule: self.name(),
                                        message: format!(
                                            "Literal '{}' should be '{}'",
                                            found, lit
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

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut result = String::with_capacity(source.len());
        let mut changed = false;
        let mut i = 0;

        while i < len {
            if skip_map.is_code(i) && is_word_char(bytes[i]) {
                let preceded_by_word = i > 0 && is_word_char(bytes[i - 1]);
                if !preceded_by_word {
                    let word_start = i;
                    let mut j = i;
                    while j < len && is_word_char(bytes[j]) {
                        j += 1;
                    }
                    let word_end = j;

                    let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));
                    let word_bytes = &bytes[word_start..word_end];

                    if all_code {
                        let mut replaced = false;

                        for &lit in LITERALS {
                            if lit.len() == word_bytes.len()
                                && lit
                                    .bytes()
                                    .zip(word_bytes.iter())
                                    .all(|(a, &b)| a.eq_ignore_ascii_case(&b))
                            {
                                let already_upper = word_bytes
                                    .iter()
                                    .all(|b| b.is_ascii_uppercase() || !b.is_ascii_alphabetic());

                                if !already_upper {
                                    result.push_str(lit);
                                    changed = true;
                                } else {
                                    result.push_str(
                                        std::str::from_utf8(word_bytes).unwrap_or("?"),
                                    );
                                }
                                replaced = true;
                                break;
                            }
                        }

                        if !replaced {
                            result.push_str(std::str::from_utf8(word_bytes).unwrap_or("?"));
                        }
                    } else {
                        result.push_str(std::str::from_utf8(word_bytes).unwrap_or("?"));
                    }

                    i = word_end;
                    continue;
                }
            }

            // Copy the byte as-is (non-word or skipped).
            // Safety: we only push valid UTF-8 since we copy individual bytes
            // from a valid UTF-8 source and only enter word branches when all
            // bytes are ASCII.
            result.push(bytes[i] as char);
            i += 1;
        }

        if changed {
            Some(result)
        } else {
            None
        }
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
