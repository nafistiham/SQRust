use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct ExplicitAlias;

impl Rule for ExplicitAlias {
    fn name(&self) -> &'static str {
        "Convention/ExplicitAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();

        // SQL keywords that can follow a table reference (not an alias)
        let non_alias_keywords: &[&[u8]] = &[
            b"WHERE", b"ON", b"SET", b"GROUP", b"ORDER", b"HAVING", b"LIMIT",
            b"UNION", b"INTERSECT", b"EXCEPT", b"JOIN", b"INNER", b"LEFT",
            b"RIGHT", b"FULL", b"OUTER", b"CROSS", b"LATERAL", b"USING",
            b"FETCH", b"OFFSET", b"FOR", b"INTO", b"VALUES", b"RETURNING",
        ];

        let mut i = 0;
        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Look for FROM or JOIN keyword
            if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
                i += 1;
                continue;
            }

            // Read word
            let ws = i;
            let mut we = i;
            while we < len && is_word_char(bytes[we]) {
                we += 1;
            }
            let word = &bytes[ws..we];

            let is_from = word.eq_ignore_ascii_case(b"FROM");
            let is_join = word.len() >= 4 && {
                let suffix = &word[word.len() - 4..];
                suffix.eq_ignore_ascii_case(b"JOIN")
            };

            if is_from || is_join {
                // Skip whitespace after FROM/JOIN
                let mut j = we;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                    j += 1;
                }
                if j >= len || !skip.is_code(j) {
                    i = we;
                    continue;
                }

                // Read table reference: either a word (table name) or a '(' (subquery)
                let table_end;
                if bytes[j] == b'(' {
                    // Skip over the parenthesized subquery
                    let mut depth = 0usize;
                    let mut k = j;
                    while k < len {
                        if skip.is_code(k) {
                            if bytes[k] == b'(' { depth += 1; }
                            else if bytes[k] == b')' {
                                depth -= 1;
                                if depth == 0 { k += 1; break; }
                            }
                        }
                        k += 1;
                    }
                    table_end = k;
                } else {
                    // Read table name (possibly schema.name)
                    let mut k = j;
                    while k < len && (is_word_char(bytes[k]) || bytes[k] == b'.') {
                        k += 1;
                    }
                    table_end = k;
                }

                if table_end == 0 || table_end >= len {
                    i = we;
                    continue;
                }

                // Skip whitespace after table/subquery
                let mut k = table_end;
                while k < len && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }

                // Check what's next
                if k >= len || !skip.is_code(k) || bytes[k] == b'\n' || bytes[k] == b'\r' || bytes[k] == b',' || bytes[k] == b')' || bytes[k] == b';' {
                    i = we;
                    continue;
                }

                // Check for AS keyword
                if is_word_char(bytes[k]) {
                    let as_start = k;
                    let mut ae = k;
                    while ae < len && is_word_char(bytes[ae]) {
                        ae += 1;
                    }
                    let next_word = &bytes[as_start..ae];

                    if next_word.eq_ignore_ascii_case(b"AS") {
                        // Good — explicit alias with AS
                        i = ae;
                        continue;
                    }

                    // Check if it's a non-alias keyword
                    let is_non_alias = non_alias_keywords.iter().any(|kw| next_word.eq_ignore_ascii_case(kw));
                    if is_non_alias {
                        i = we;
                        continue;
                    }

                    // It's an implicit alias — flag it
                    let (line, col) = offset_to_line_col(source, as_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Table alias '{}' should use the AS keyword",
                            String::from_utf8_lossy(next_word)
                        ),
                        line,
                        col,
                    });
                    i = ae;
                    continue;
                }
            }

            i = we;
        }

        diags
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
