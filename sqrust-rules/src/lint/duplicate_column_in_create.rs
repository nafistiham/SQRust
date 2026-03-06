use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::Statement;
use std::collections::{HashMap, HashSet};

pub struct DuplicateColumnInCreate;

impl Rule for DuplicateColumnInCreate {
    fn name(&self) -> &'static str {
        "Lint/DuplicateColumnInCreate"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::CreateTable(create_table) = stmt {
                let columns = &create_table.columns;

                // Count occurrences of each column name (case-insensitive).
                let mut seen: HashMap<String, usize> = HashMap::new();
                // Track which duplicates were already reported (avoid duplicate reports for
                // three-or-more occurrences of the same name).
                let mut reported: HashSet<String> = HashSet::new();

                for col_def in columns {
                    let lower = col_def.name.value.to_lowercase();
                    let count = seen.entry(lower.clone()).or_insert(0);
                    *count += 1;

                    if *count == 2 && !reported.contains(&lower) {
                        reported.insert(lower.clone());

                        // Find the position of the second occurrence of this column name in source.
                        let (line, col) =
                            find_second_occurrence(&ctx.source, &col_def.name.value);
                        diags.push(Diagnostic {
                            rule: "Lint/DuplicateColumnInCreate",
                            message: format!(
                                "Column '{}' is defined more than once in CREATE TABLE",
                                lower
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

/// Finds the second occurrence of `name` (case-insensitive, whole-word) in `source`
/// and returns its 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_second_occurrence(source: &str, name: &str) -> (usize, usize) {
    let source_lower = source.to_lowercase();
    let name_lower = name.to_lowercase();
    let name_len = name_lower.len();
    let bytes = source_lower.as_bytes();
    let src_len = bytes.len();

    let mut search_from = 0usize;
    let mut occurrences_found = 0usize;

    while search_from < src_len {
        let Some(rel) = source_lower[search_from..].find(&name_lower) else {
            break;
        };
        let abs = search_from + rel;

        // Word-boundary check.
        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + name_len;
        let after_ok = after >= src_len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            occurrences_found += 1;
            if occurrences_found == 2 {
                return offset_to_line_col(source, abs);
            }
        }

        search_from = abs + 1;
    }

    // Fallback: return position of first occurrence if second not found distinctly.
    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
