use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct PreferAnsiTrim;

impl Rule for PreferAnsiTrim {
    fn name(&self) -> &'static str {
        "Convention/PreferAnsiTrim"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let mut diags = Vec::new();

        for (keyword, message) in &[
            (
                "ltrim",
                "LTRIM() is non-standard; use TRIM(LEADING ' ' FROM col) or TRIM(col) for portable SQL",
            ),
            (
                "rtrim",
                "RTRIM() is non-standard; use TRIM(TRAILING ' ' FROM col) or TRIM(col) for portable SQL",
            ),
        ] {
            let lower = source.to_lowercase();
            let kw_len = keyword.len();
            let bytes = lower.as_bytes();
            let len = bytes.len();
            let mut i = 0;
            while i + kw_len <= len {
                if !skip.contains(&i) && lower[i..].starts_with(keyword) {
                    let all_code = (i..i + kw_len).all(|k| !skip.contains(&k));
                    if all_code {
                        let before_ok = i == 0 || {
                            let b = bytes[i - 1];
                            !b.is_ascii_alphanumeric() && b != b'_'
                        };
                        let after_pos = i + kw_len;
                        // Must be followed by '(' to be a function call
                        let after_ok = after_pos < len && bytes[after_pos] == b'(';
                        if before_ok && after_ok {
                            let (line, col) = offset_to_line_col(source, i);
                            diags.push(Diagnostic {
                                rule: self.name(),
                                message: (*message).to_string(),
                                line,
                                col,
                            });
                        }
                    }
                }
                i += 1;
            }
        }

        diags.sort_by_key(|d| (d.line, d.col));
        diags
    }
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
