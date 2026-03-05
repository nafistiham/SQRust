use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;

pub struct DeleteWithoutWhere;

impl Rule for DeleteWithoutWhere {
    fn name(&self) -> &'static str {
        "Lint/DeleteWithoutWhere"
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
            if let Statement::Delete(delete) = stmt {
                if delete.selection.is_none() {
                    // Find the position of the DELETE keyword in source (case-insensitive,
                    // word-boundary check).
                    let (line, col) = find_keyword_position(source, &source_upper, "DELETE");
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "DELETE without WHERE clause will delete all rows".to_string(),
                        line,
                        col,
                    });
                }
            }
        }

        diags
    }
}

/// Finds the 1-indexed (line, col) of the first occurrence of `keyword` (already uppercased)
/// in `source_upper` that has word boundaries on both sides.
/// Falls back to (1, 1) if not found.
fn find_keyword_position(source: &str, source_upper: &str, keyword: &str) -> (usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut search_from = 0usize;
    while search_from < text_len {
        let Some(rel) = source_upper[search_from..].find(keyword) else {
            break;
        };
        let abs = search_from + rel;

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
        search_from = abs + 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
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
