use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct NoIlike;

const MESSAGE: &str =
    "ILIKE is PostgreSQL-specific; use LOWER(col) LIKE LOWER(pattern) for portable case-insensitive matching";

impl Rule for NoIlike {
    fn name(&self) -> &'static str {
        "Convention/NoIlike"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
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

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if source.is_empty() {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let lower = source.to_lowercase();
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let keyword = b"ilike";
    let kw_len = keyword.len();
    let mut diags = Vec::new();
    let mut i = 0;

    while i + kw_len <= len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        if &bytes[i..i + kw_len] != keyword {
            i += 1;
            continue;
        }

        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        // Word boundary after
        let after_pos = i + kw_len;
        let after_ok = after_pos >= len || !is_word_char(bytes[after_pos]);

        if before_ok && after_ok {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: MESSAGE.to_string(),
                line,
                col,
            });
            i = after_pos;
        } else {
            i += 1;
        }
    }

    diags
}
