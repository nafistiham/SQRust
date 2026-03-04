use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::path::PathBuf;

/// A single lint violation produced by a Rule.
pub struct Diagnostic {
    pub rule: &'static str,
    pub message: String,
    /// 1-indexed line number
    pub line: usize,
    /// 1-indexed column of the violation
    pub col: usize,
}

/// All information a Rule needs to check one file.
pub struct FileContext {
    pub path: PathBuf,
    pub source: String,
    /// Parsed SQL statements. Empty if the file could not be parsed.
    pub statements: Vec<Statement>,
    /// Parse error messages, if parsing failed.
    pub parse_errors: Vec<String>,
}

impl FileContext {
    pub fn from_source(source: &str, path: &str) -> Self {
        let dialect = GenericDialect {};
        let (statements, parse_errors) = match Parser::parse_sql(&dialect, source) {
            Ok(stmts) => (stmts, Vec::new()),
            Err(e) => (Vec::new(), vec![e.to_string()]),
        };
        FileContext {
            path: PathBuf::from(path),
            source: source.to_string(),
            statements,
            parse_errors,
        }
    }

    /// Returns (1-indexed line number, line content) for each line.
    pub fn lines(&self) -> impl Iterator<Item = (usize, &str)> {
        self.source.lines().enumerate().map(|(i, line)| (i + 1, line))
    }
}

/// Every lint rule implements this trait.
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic>;
    /// Returns the fixed source if this rule supports auto-fix, None otherwise.
    fn fix(&self, _ctx: &FileContext) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sql_populates_statements() {
        let ctx = FileContext::from_source("SELECT 1; SELECT 2;", "t.sql");
        assert_eq!(ctx.statements.len(), 2);
        assert!(ctx.parse_errors.is_empty());
    }

    #[test]
    fn invalid_sql_stores_parse_error() {
        let ctx = FileContext::from_source("SELECT FROM FROM", "t.sql");
        assert!(ctx.statements.is_empty());
        assert!(!ctx.parse_errors.is_empty());
    }

    #[test]
    fn empty_sql_produces_no_statements_and_no_errors() {
        let ctx = FileContext::from_source("", "t.sql");
        assert!(ctx.statements.is_empty());
        assert!(ctx.parse_errors.is_empty());
    }

    #[test]
    fn lines_still_works_after_ast_addition() {
        let ctx = FileContext::from_source("SELECT 1\nFROM t\n", "t.sql");
        let lines: Vec<_> = ctx.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (1, "SELECT 1"));
        assert_eq!(lines[1], (2, "FROM t"));
    }
}
