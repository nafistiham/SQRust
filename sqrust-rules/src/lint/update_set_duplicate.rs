use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{AssignmentTarget, ObjectName, Statement};
use std::collections::HashMap;

pub struct UpdateSetDuplicate;

impl Rule for UpdateSetDuplicate {
    fn name(&self) -> &'static str {
        "Lint/UpdateSetDuplicate"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Update { assignments, .. } = stmt {
                // Count occurrences of each column name (lowercased).
                let mut counts: HashMap<String, usize> = HashMap::new();
                // Preserve insertion order for deterministic diagnostics.
                let mut order: Vec<String> = Vec::new();

                for assignment in assignments {
                    if let AssignmentTarget::ColumnName(col_name) = &assignment.target {
                        let name = extract_column_name(col_name);
                        let lower = name.to_lowercase();
                        let entry = counts.entry(lower.clone()).or_insert(0);
                        *entry += 1;
                        if *entry == 1 {
                            order.push(lower);
                        }
                    }
                }

                // Emit one diagnostic per duplicated column name.
                for col_lower in &order {
                    if counts[col_lower] > 1 {
                        // Find the position of the second occurrence of `col = ` in source.
                        let (line, col) = find_second_occurrence(&ctx.source, col_lower);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: format!(
                                "Column '{}' appears more than once in UPDATE SET clause",
                                col_lower
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

/// Extracts the column name from an ObjectName (last ident, preserving original case).
fn extract_column_name(obj: &ObjectName) -> &str {
    obj.0
        .last()
        .map(|id| id.value.as_str())
        .unwrap_or("")
}

/// Finds the 1-indexed (line, col) of the second occurrence of
/// `col_name\s*=` (case-insensitive) outside strings/comments in `source`.
/// Falls back to (1, 1) if fewer than two occurrences are found.
fn find_second_occurrence(source: &str, col_name: &str) -> (usize, usize) {
    let source_lower = source.to_lowercase();
    let name_lower = col_name.to_lowercase();
    let name_len = name_lower.len();
    let bytes = source_lower.as_bytes();
    let len = bytes.len();

    let mut found = 0usize;
    let mut search_from = 0usize;

    while search_from < len {
        let Some(rel) = source_lower[search_from..].find(&name_lower) else {
            break;
        };
        let abs = search_from + rel;

        // Word boundary check.
        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after_name = abs + name_len;
        let after_ok = after_name >= len || {
            let b = bytes[after_name];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            // Verify that `=` follows (optionally with whitespace), indicating an assignment.
            let after_ws = skip_whitespace_in(bytes, after_name, len);
            let is_assignment = after_ws < len && bytes[after_ws] == b'=';

            if is_assignment {
                found += 1;
                if found == 2 {
                    return offset_to_line_col(source, abs);
                }
            }
        }

        search_from = abs + 1;
    }

    (1, 1)
}

fn skip_whitespace_in(bytes: &[u8], mut pos: usize, len: usize) -> usize {
    while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' || bytes[pos] == b'\r') {
        pos += 1;
    }
    pos
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
