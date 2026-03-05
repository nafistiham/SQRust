use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Statement};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct TooManyCtes {
    /// Maximum number of CTEs allowed per query. Queries with more CTEs than
    /// this are flagged.
    pub max_ctes: usize,
}

impl Default for TooManyCtes {
    fn default() -> Self {
        TooManyCtes { max_ctes: 5 }
    }
}

impl Rule for TooManyCtes {
    fn name(&self) -> &'static str {
        "TooManyCtes"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                if let Some(with) = &query.with {
                    let n = with.cte_tables.len();
                    if n > self.max_ctes {
                        let (line, col) = find_keyword_pos(&ctx.source, "WITH");
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: format!(
                                "Query defines {n} CTEs, exceeding the maximum of {max}",
                                n = n,
                                max = self.max_ctes,
                            ),
                            line,
                            col,
                        });
                    }
                }
            }
        }

        diags
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of a keyword (case-insensitive, word-boundary,
/// outside strings/comments) in `source`. Returns a 1-indexed (line, col)
/// pair. Falls back to (1, 1) if not found.
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match.
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw_upper.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            // Word boundary after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                return line_col(source, i);
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
