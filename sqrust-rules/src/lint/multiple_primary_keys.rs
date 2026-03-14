use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{ColumnOption, Statement, TableConstraint};

pub struct MultiplePrimaryKeys;

impl Rule for MultiplePrimaryKeys {
    fn name(&self) -> &'static str {
        "Lint/MultiplePrimaryKeys"
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
            if let Statement::CreateTable(create_table) = stmt {
                let pk_count = count_primary_keys(create_table);
                if pk_count > 1 {
                    // Derive table name for the message.
                    let table_name = create_table.name.to_string();
                    let (line, col) =
                        find_create_table_position(source, &source_upper, &table_name);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Table '{}' defines {} PRIMARY KEY constraints — a table can have only one primary key",
                            table_name, pk_count
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

/// Count the total number of PRIMARY KEY definitions in the CREATE TABLE:
/// - column-level: `col INT PRIMARY KEY`
/// - table-level: `PRIMARY KEY (col, ...)`
fn count_primary_keys(create_table: &sqlparser::ast::CreateTable) -> usize {
    let mut count = 0usize;

    // Column-level PRIMARY KEY.
    for col in &create_table.columns {
        for option_def in &col.options {
            if let ColumnOption::Unique { is_primary, .. } = &option_def.option {
                if *is_primary {
                    count += 1;
                }
            }
        }
    }

    // Table-level PRIMARY KEY constraint.
    for constraint in &create_table.constraints {
        if matches!(constraint, TableConstraint::PrimaryKey { .. }) {
            count += 1;
        }
    }

    count
}

/// Find the (line, col) of the `CREATE TABLE <name>` occurrence in source.
/// Scans for "CREATE" followed eventually by the table name on the same
/// statement. Falls back to (1, 1).
fn find_create_table_position(source: &str, source_upper: &str, table_name: &str) -> (usize, usize) {
    let table_upper = table_name.to_uppercase();
    let bytes = source_upper.as_bytes();
    let len = bytes.len();
    let create_kw = b"CREATE";
    let create_len = create_kw.len();
    let mut search_from = 0usize;

    while search_from < len {
        let Some(rel) = source_upper[search_from..].find("CREATE") else {
            break;
        };
        let abs = search_from + rel;

        // Word boundary before CREATE
        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after_create = abs + create_len;
        let after_ok = after_create >= len || {
            let b = bytes[after_create];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            // Look for the table name within a reasonable window (512 bytes)
            let window_end = (abs + 512).min(len);
            let window = &source_upper[abs..window_end];
            if window.contains(&table_upper as &str) {
                return offset_to_line_col(source, abs);
            }
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
