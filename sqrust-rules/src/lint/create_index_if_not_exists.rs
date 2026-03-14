use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{CreateIndex, Statement};

pub struct CreateIndexIfNotExists;

impl Rule for CreateIndexIfNotExists {
    fn name(&self) -> &'static str {
        "Lint/CreateIndexIfNotExists"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();

        // Track how many CREATE INDEX violations we have found so we can locate each
        // occurrence in the source text.
        let mut create_index_occurrence: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::CreateIndex(CreateIndex {
                name,
                if_not_exists,
                ..
            }) = stmt
            {
                if !if_not_exists {
                    let (line, col) = find_nth_create_index(
                        source,
                        &source_upper,
                        create_index_occurrence,
                    );
                    create_index_occurrence += 1;

                    let index_name = name
                        .as_ref()
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "unnamed".to_string());

                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "CREATE INDEX '{}' is missing IF NOT EXISTS — use 'CREATE INDEX IF NOT EXISTS ...' for idempotent index creation",
                            index_name
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        diags
    }
}

/// Finds the 1-indexed (line, col) of the `nth` (0-indexed) `CREATE INDEX`
/// pair in `source`, skipping over occurrences where `CREATE` is not followed
/// by optional whitespace+`UNIQUE`+whitespace and then `INDEX`.
/// Falls back to the nth bare `CREATE` position if the sequence-matching
/// fails, and ultimately to (1, 1).
fn find_nth_create_index(
    source: &str,
    source_upper: &str,
    nth: usize,
) -> (usize, usize) {
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut count = 0usize;
    let mut search_from = 0usize;

    while search_from < text_len {
        let Some(rel) = source_upper[search_from..].find("CREATE") else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        let after_create = abs + "CREATE".len();
        let after_ok = after_create >= text_len
            || {
                let b = bytes[after_create];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok && after_ok {
            // Now look ahead past optional whitespace and optional UNIQUE keyword
            // to see if INDEX follows.
            if is_create_index_at(source_upper, after_create, text_len) {
                if count == nth {
                    return offset_to_line_col(source, abs);
                }
                count += 1;
            }
        }
        search_from = abs + 1;
    }

    (1, 1)
}

/// Returns true if, starting at `pos` in `source_upper`, we find (optional
/// whitespace) then optionally (UNIQUE + whitespace) then INDEX at a word
/// boundary.
fn is_create_index_at(source_upper: &str, pos: usize, len: usize) -> bool {
    let bytes = source_upper.as_bytes();
    let mut i = pos;

    // Skip whitespace
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    // Optional UNIQUE keyword
    if i + 6 <= len && &source_upper[i..i + 6] == "UNIQUE" {
        let after_unique = i + 6;
        let word_boundary = after_unique >= len
            || {
                let b = bytes[after_unique];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        if word_boundary {
            i = after_unique;
            // Skip whitespace after UNIQUE
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }
    }

    // Must find INDEX at word boundary
    if i + 5 > len {
        return false;
    }
    if &source_upper[i..i + 5] != "INDEX" {
        return false;
    }
    let after_index = i + 5;
    after_index >= len
        || {
            let b = bytes[after_index];
            !b.is_ascii_alphanumeric() && b != b'_'
        }
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
