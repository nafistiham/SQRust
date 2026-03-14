use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;

pub struct UpdateWithJoin;

impl Rule for UpdateWithJoin {
    fn name(&self) -> &'static str {
        "Structure/UpdateWithJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        for stmt in &ctx.statements {
            if let Statement::Update { table, from, .. } = stmt {
                // Flag when the table itself has JOINs (e.g. MySQL-style
                // `UPDATE t JOIN s ON … SET …`) OR when a FROM clause is
                // present (PostgreSQL/SQL-Server `UPDATE t … FROM s …`).
                let table_has_join = !table.joins.is_empty();
                let has_from = from.is_some();

                if table_has_join || has_from {
                    let (line, col) =
                        find_keyword_position(source, &source_upper, "UPDATE");
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message:
                            "UPDATE with JOIN/FROM syntax is SQL Server/PostgreSQL-specific \
                             — use a correlated subquery for portability"
                                .to_string(),
                        line,
                        col,
                    });
                }
            }
        }

        diags
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of `keyword` (already upper-cased) in
/// `source_upper` that has word boundaries on both sides. Returns a 1-indexed
/// (line, col) pair. Falls back to (1, 1) if not found.
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
