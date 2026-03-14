use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct InsertOverwrite;

impl Rule for InsertOverwrite {
    fn name(&self) -> &'static str {
        "Lint/InsertOverwrite"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let mut diags = Vec::new();

        for (line, col) in find_two_word_keyword(source, "insert", "overwrite", &skip) {
            diags.push(Diagnostic {
                rule: self.name(),
                message: "INSERT OVERWRITE is Hive/Spark SQL-specific syntax; use standard INSERT INTO or CREATE TABLE AS SELECT"
                    .to_string(),
                line,
                col,
            });
        }

        diags.sort_by_key(|d| (d.line, d.col));
        diags
    }
}

/// Builds a set of byte offsets that are inside string literals or line comments.
fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

/// Finds occurrences of `word1` followed by whitespace and `word2` (case-insensitive),
/// both at word boundaries, skipping positions in the skip set.
/// Returns 1-indexed (line, col) of `word1` for each match.
fn find_two_word_keyword(
    source: &str,
    word1: &str,
    word2: &str,
    skip: &HashSet<usize>,
) -> Vec<(usize, usize)> {
    let lower = source.to_lowercase();
    let w1_len = word1.len();
    let w2_len = word2.len();
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i + w1_len <= len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        if !lower[i..].starts_with(word1) {
            i += 1;
            continue;
        }

        // Word boundary before word1
        let before_ok = i == 0
            || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        // Word boundary after word1
        let after_w1 = i + w1_len;
        let after_w1_ok = after_w1 < len && {
            let b = bytes[after_w1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_w1_ok {
            // Skip whitespace between word1 and word2
            let mut j = after_w1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }

            if j + w2_len <= len && !skip.contains(&j) && lower[j..].starts_with(word2) {
                // Word boundary after word2
                let after_w2 = j + w2_len;
                let after_w2_ok = after_w2 >= len
                    || {
                        let b = bytes[after_w2];
                        !b.is_ascii_alphanumeric() && b != b'_'
                    };

                if after_w2_ok {
                    let (line, col) = offset_to_line_col(source, i);
                    results.push((line, col));
                }
            }
        }

        i += 1;
    }

    results
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
