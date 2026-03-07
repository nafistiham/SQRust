use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MaxIdentifierLength {
    pub max_length: usize,
}

impl Default for MaxIdentifierLength {
    fn default() -> Self {
        MaxIdentifierLength { max_length: 30 }
    }
}

impl Rule for MaxIdentifierLength {
    fn name(&self) -> &'static str {
        "Layout/MaxIdentifierLength"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let max = self.max_length;
        let mut diags = Vec::new();

        // Scan the source text for identifiers using a simple tokenizer.
        // We skip string literals and only look at unquoted and quoted identifiers.
        let src = &ctx.source;
        let bytes = src.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let ch = bytes[i];

            // Skip string literals (single-quoted).
            if ch == b'\'' {
                i += 1;
                while i < len && bytes[i] != b'\'' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1; // closing quote
                continue;
            }

            // Skip line comments.
            if i + 1 < len && ch == b'-' && bytes[i + 1] == b'-' {
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Quoted identifiers ("..." or `...`).
            if ch == b'"' || ch == b'`' {
                let close = ch;
                let start = i;
                i += 1;
                let content_start = i;
                while i < len && bytes[i] != close {
                    i += 1;
                }
                let content = &src[content_start..i];
                i += 1; // closing quote
                if content.len() > max {
                    let (line, col) = offset_to_line_col(src, start);
                    diags.push(Diagnostic {
                        rule: "Layout/MaxIdentifierLength",
                        message: format!(
                            "Identifier '{}' is {} characters long; maximum is {}",
                            content,
                            content.len(),
                            max
                        ),
                        line,
                        col,
                    });
                }
                continue;
            }

            // Unquoted identifiers: start with letter or underscore.
            if ch.is_ascii_alphabetic() || ch == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = &src[start..i];
                // Skip SQL keywords.
                if !is_keyword(ident) && ident.len() > max {
                    let (line, col) = offset_to_line_col(src, start);
                    diags.push(Diagnostic {
                        rule: "Layout/MaxIdentifierLength",
                        message: format!(
                            "Identifier '{}' is {} characters long; maximum is {}",
                            ident,
                            ident.len(),
                            max
                        ),
                        line,
                        col,
                    });
                }
                continue;
            }

            i += 1;
        }

        diags
    }
}

fn is_keyword(s: &str) -> bool {
    const KEYWORDS: &[&str] = &[
        "SELECT", "FROM", "WHERE", "JOIN", "INNER", "LEFT", "RIGHT", "FULL",
        "OUTER", "ON", "AND", "OR", "NOT", "IN", "IS", "NULL", "AS", "BY",
        "GROUP", "ORDER", "HAVING", "LIMIT", "OFFSET", "UNION", "ALL",
        "DISTINCT", "INSERT", "INTO", "UPDATE", "SET", "DELETE", "CREATE",
        "TABLE", "ALTER", "DROP", "WITH", "CASE", "WHEN", "THEN", "ELSE",
        "END", "EXISTS", "BETWEEN", "LIKE", "ASC", "DESC", "PRIMARY", "KEY",
        "FOREIGN", "REFERENCES", "INDEX", "VIEW", "CROSS", "NATURAL",
        "USING", "VALUES", "DEFAULT", "CONSTRAINT", "UNIQUE", "CHECK",
        "RETURNS", "RETURN", "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
        "TRUE", "FALSE", "EXCEPT", "INTERSECT", "RECURSIVE", "LATERAL",
        "REPLACE", "IF", "COUNT", "SUM", "AVG", "MIN", "MAX", "ABS", "YEAR",
        "MONTH", "DAY", "UPPER", "LOWER", "LENGTH", "TRIM", "COALESCE",
        "NULLIF", "CAST", "CONVERT", "SUBSTRING", "CONCAT", "REPLACE",
        "OVER", "PARTITION", "ROWS", "RANGE", "UNBOUNDED", "PRECEDING",
        "FOLLOWING", "CURRENT", "ROW",
    ];
    KEYWORDS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(s))
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
