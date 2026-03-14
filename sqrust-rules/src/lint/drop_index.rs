use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{ObjectType, Statement};

pub struct DropIndex;

impl Rule for DropIndex {
    fn name(&self) -> &'static str {
        "Lint/DropIndex"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        let mut drop_index_occurrence: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Drop {
                object_type,
                if_exists,
                ..
            } = stmt
            {
                if *object_type == ObjectType::Index && !if_exists {
                    let (line, col) =
                        find_nth_keyword(source, &source_upper, "DROP", drop_index_occurrence);
                    drop_index_occurrence += 1;
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "DROP INDEX without IF EXISTS will fail if the index does not \
                                  exist — use DROP INDEX IF EXISTS"
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

/// Finds the 1-indexed (line, col) of the `nth` (0-indexed) word-boundary occurrence
/// of `keyword` (already uppercased) in `source_upper`.
/// Falls back to (1, 1) if not found.
fn find_nth_keyword(
    source: &str,
    source_upper: &str,
    keyword: &str,
    nth: usize,
) -> (usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut count = 0usize;
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
            if count == nth {
                return offset_to_line_col(source, abs);
            }
            count += 1;
        }
        search_from = abs + 1;
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
