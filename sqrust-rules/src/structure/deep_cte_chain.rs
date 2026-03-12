use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;

pub struct DeepCteChain {
    /// Maximum CTE chain depth allowed. Chains longer than this are flagged.
    pub max_depth: usize,
}

impl Default for DeepCteChain {
    fn default() -> Self {
        Self { max_depth: 5 }
    }
}

impl Rule for DeepCteChain {
    fn name(&self) -> &'static str {
        "Structure/DeepCteChain"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                if let Some(with) = &query.with {
                    let cte_names: Vec<String> = with
                        .cte_tables
                        .iter()
                        .map(|cte| cte.alias.name.value.to_uppercase())
                        .collect();

                    // For each CTE, produce a string representation of its body
                    // using the Display impl on the query AST node.
                    let cte_bodies: Vec<String> = with
                        .cte_tables
                        .iter()
                        .map(|cte| format!("{}", cte.query).to_uppercase())
                        .collect();

                    // depth[i] = length of the longest chain ending at CTE i.
                    // A CTE at position i can only reference CTEs at positions 0..i-1
                    // (standard SQL: earlier CTEs in a WITH clause).
                    let mut depths: Vec<usize> = vec![1; cte_names.len()];

                    for i in 1..cte_names.len() {
                        for j in 0..i {
                            if contains_word(&cte_bodies[i], &cte_names[j]) {
                                let candidate = depths[j] + 1;
                                if candidate > depths[i] {
                                    depths[i] = candidate;
                                }
                            }
                        }
                    }

                    let max_chain = depths.iter().copied().max().unwrap_or(0);

                    if max_chain > self.max_depth {
                        let (line, col) = find_with_pos(&ctx.source);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: format!(
                                "CTE chain depth {} exceeds maximum of {} — consider breaking into separate queries or views",
                                max_chain, self.max_depth
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

/// Returns true if `text` contains `word` as a whole word (alphanumeric /
/// underscore word boundaries on both sides). Both `text` and `word` must
/// already be uppercased by the caller.
fn contains_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let word_bytes = word.as_bytes();
    let word_len = word_bytes.len();
    let text_len = bytes.len();

    let mut pos = 0usize;
    while pos + word_len <= text_len {
        let Some(rel) = text[pos..].find(word) else {
            break;
        };
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
    }

    false
}

/// Find the byte offset of the first `WITH` keyword in source (case-insensitive,
/// word-boundary) and return a 1-indexed (line, col) pair. Falls back to (1, 1).
fn find_with_pos(source: &str) -> (usize, usize) {
    let source_upper = source.to_uppercase();
    let bytes = source_upper.as_bytes();
    let kw = b"WITH";
    let kw_len = kw.len();
    let text_len = bytes.len();

    let mut pos = 0usize;
    while pos + kw_len <= text_len {
        let Some(rel) = source_upper[pos..].find("WITH") else {
            break;
        };
        let abs = pos + rel;

        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        let after = abs + kw_len;
        let after_ok = after >= text_len
            || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok && after_ok {
            return offset_to_line_col(source, abs);
        }
        pos = abs + 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
