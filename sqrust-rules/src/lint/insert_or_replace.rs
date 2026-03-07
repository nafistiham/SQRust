use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{SqliteOnConflict, Statement};

pub struct InsertOrReplace;

impl Rule for InsertOrReplace {
    fn name(&self) -> &'static str {
        "Lint/InsertOrReplace"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST is incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        // Advance search_from past each REPLACE keyword we flag or skip.
        let mut search_from = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Insert(insert) = stmt {
                // Detect REPLACE INTO (MySQL): replace_into = true
                let is_replace_into = insert.replace_into;

                // Detect INSERT OR REPLACE INTO (SQLite):
                // or = Some(SqliteOnConflict::Replace)
                let is_insert_or_replace = matches!(insert.or, Some(SqliteOnConflict::Replace));

                if is_replace_into || is_insert_or_replace {
                    let (line, col) =
                        find_keyword_position(source, &source_upper, "REPLACE", &mut search_from);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message:
                            "INSERT OR REPLACE/REPLACE INTO silently deletes and re-inserts rows; prefer INSERT ... ON CONFLICT"
                                .to_string(),
                        line,
                        col,
                    });
                } else {
                    // A regular INSERT — advance past "INSERT" so the next
                    // search for REPLACE does not get confused.
                    advance_past_keyword(source, &source_upper, "INSERT", &mut search_from);
                }
            }
        }

        diags
    }
}

/// Finds the 1-indexed (line, col) of the next word-boundary occurrence of
/// `keyword` (already uppercased) in `source_upper` starting from `search_from`.
/// Updates `search_from` to just past the match. Falls back to (1, 1).
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

/// Advance `search_from` past the next word-boundary occurrence of `keyword`
/// without emitting a diagnostic.
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
/// Requires `keyword` to be already uppercased.
fn find_keyword_inner(
    source: &str,
    source_upper: &str,
    keyword: &str,
    start: usize,
) -> (usize, usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut pos = start;
    while pos < text_len {
        let Some(rel) = source_upper[pos..].find(keyword) else {
            break;
        };
        let abs = pos + rel;

        // Word-boundary check: character before keyword.
        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        // Word-boundary check: character after keyword.
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
        pos = abs + 1;
    }

    (1, 1, start)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
