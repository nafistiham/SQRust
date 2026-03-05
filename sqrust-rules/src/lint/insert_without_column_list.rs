use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;

pub struct InsertWithoutColumnList;

impl Rule for InsertWithoutColumnList {
    fn name(&self) -> &'static str {
        "Lint/InsertWithoutColumnList"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST is incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        // Track byte offset so we can report the position of each INSERT.
        // When there are multiple INSERT statements, we search for the
        // next INSERT keyword from after the previous one.
        let mut search_from = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Insert(insert) = stmt {
                if insert.columns.is_empty() {
                    let (line, col) =
                        find_keyword_position(source, &source_upper, "INSERT", &mut search_from);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "INSERT statement missing explicit column list; specify columns for safety"
                            .to_string(),
                        line,
                        col,
                    });
                } else {
                    // Has columns — still advance past this INSERT keyword so
                    // the next search starts after it.
                    advance_past_keyword(source, &source_upper, "INSERT", &mut search_from);
                }
            }
        }

        diags
    }
}

/// Finds the 1-indexed (line, col) of the next occurrence of `keyword`
/// (already uppercased) in `source_upper` starting from `search_from`,
/// with word boundaries on both sides.
/// Updates `search_from` to point past the matched keyword.
/// Falls back to (1, 1) if not found.
fn find_keyword_position(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) -> (usize, usize) {
    let (line, col, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
    (line, col)
}

/// Advance `search_from` past the next occurrence of `keyword` without
/// recording a diagnostic.
fn advance_past_keyword(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) {
    let (_, _, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
}

/// Core search: returns (line, col, next_search_from).
fn find_keyword_inner(
    source: &str,
    source_upper: &str,
    keyword: &str,
    start: usize,
) -> (usize, usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut search_from = start;
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
            let (line, col) = offset_to_line_col(source, abs);
            return (line, col, after);
        }
        search_from = abs + 1;
    }

    (1, 1, start)
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
