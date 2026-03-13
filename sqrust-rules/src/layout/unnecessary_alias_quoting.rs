use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct UnnecessaryAliasQuoting;

/// SQL reserved keywords that should NOT be flagged even if they're simple
/// identifiers. Quoting these is typically intentional to avoid parser
/// ambiguity.
const RESERVED_KEYWORDS: &[&str] = &[
    "select", "from", "where", "group", "order", "having", "limit", "join",
    "on", "as", "by", "desc", "asc", "null", "true", "false", "not", "and",
    "or", "in", "is", "like", "case", "when", "then", "else", "end", "exists",
    "between", "distinct", "all", "union", "intersect", "except", "insert",
    "update", "delete", "create", "drop", "alter", "table", "view", "index",
    "with", "recursive", "set", "into", "values", "using", "natural", "cross",
    "inner", "left", "right", "full", "outer", "over", "partition", "rows",
    "range", "unbounded", "preceding", "following", "current", "row", "filter",
    "within", "date", "time", "timestamp", "year", "month", "day", "hour",
    "minute", "second", "interval", "default", "constraint", "primary", "key",
    "foreign", "references", "unique", "check", "comment", "name", "value",
    "status", "type", "user", "level", "position", "data", "text",
];

impl Rule for UnnecessaryAliasQuoting {
    fn name(&self) -> &'static str {
        "Layout/UnnecessaryAliasQuoting"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Bail out if the file had parse errors — source-level scan could
        // produce false positives for broken SQL.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        find_violations(&ctx.source, self.name())
    }
}

/// Returns `true` if the name is a "simple" identifier that does not need
/// quoting: matches `^[a-zA-Z_][a-zA-Z0-9_]*$` and is not a reserved keyword.
fn is_simple_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    !RESERVED_KEYWORDS.contains(&lower.as_str())
}

fn byte_offset_to_line_col(bytes: &[u8], offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for i in 0..offset {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = offset - line_start + 1;
    (line, col)
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    // Scan for `AS` keyword (case-insensitive) followed by optional whitespace
    // followed by `"` or `` ` ``.
    let mut i = 0;
    while i < len {
        // Match `AS` case-insensitively.
        if i + 1 < len
            && (bytes[i] == b'A' || bytes[i] == b'a')
            && (bytes[i + 1] == b'S' || bytes[i + 1] == b's')
        {
            // Ensure `AS` is preceded by a word boundary (space, comma,
            // opening paren, or start of source).
            let preceded_by_boundary = if i == 0 {
                true
            } else {
                let prev = bytes[i - 1];
                prev == b' ' || prev == b'\t' || prev == b'\n' || prev == b',' || prev == b'('
            };

            // Ensure `AS` is followed by whitespace.
            let after_as = i + 2;
            let followed_by_space = after_as < len
                && (bytes[after_as] == b' ' || bytes[after_as] == b'\t' || bytes[after_as] == b'\n');

            if preceded_by_boundary && followed_by_space {
                // Skip whitespace after AS.
                let mut j = after_as;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                    j += 1;
                }

                // Check for double-quote or backtick.
                if j < len && (bytes[j] == b'"' || bytes[j] == b'`') {
                    let quote_char = bytes[j];
                    let close_quote = if quote_char == b'"' { b'"' } else { b'`' };
                    let alias_start = j + 1;
                    // Find closing quote.
                    let mut k = alias_start;
                    while k < len && bytes[k] != close_quote {
                        k += 1;
                    }
                    if k < len {
                        // Extract alias name.
                        let alias = match std::str::from_utf8(&bytes[alias_start..k]) {
                            Ok(s) => s,
                            Err(_) => {
                                i += 2;
                                continue;
                            }
                        };

                        if is_simple_identifier(alias) {
                            let (line, col) = byte_offset_to_line_col(bytes, j);
                            diags.push(Diagnostic {
                                rule: rule_name,
                                message: format!(
                                    "Alias '{}' is unnecessarily quoted — simple identifiers don't need quoting",
                                    alias
                                ),
                                line,
                                col,
                            });
                        }

                        i = k + 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    diags
}
