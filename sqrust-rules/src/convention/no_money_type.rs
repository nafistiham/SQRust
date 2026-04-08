use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct NoMoneyType;

const MSG_MONEY: &str = "Avoid MONEY type — use DECIMAL or NUMERIC for portability";
const MSG_SMALLMONEY: &str = "Avoid SMALLMONEY type — use DECIMAL or NUMERIC for portability";

impl Rule for NoMoneyType {
    fn name(&self) -> &'static str {
        "Convention/NoMoneyType"
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
            // Single-quoted string
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
            // Line comment
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment
            skip.insert(i);
            skip.insert(i + 1);
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip.insert(i);
                    skip.insert(i + 1);
                    i += 2;
                    break;
                } else {
                    skip.insert(i);
                    i += 1;
                }
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

struct Keyword {
    pattern: &'static [u8],
    message: &'static str,
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if source.is_empty() {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let lower = source.to_lowercase();
    let bytes = lower.as_bytes();
    let len = bytes.len();

    // Check SMALLMONEY before MONEY so the longer keyword is tested first.
    // This avoids matching "MONEY" inside "SMALLMONEY" at the same position.
    let keywords: &[Keyword] = &[
        Keyword { pattern: b"smallmoney", message: MSG_SMALLMONEY },
        Keyword { pattern: b"money",      message: MSG_MONEY },
    ];

    let mut diags = Vec::new();
    let mut i = 0;

    'outer: while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        for kw in keywords {
            let kw_len = kw.pattern.len();
            if i + kw_len > len {
                continue;
            }
            if &bytes[i..i + kw_len] != kw.pattern {
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
                    message: kw.message.to_string(),
                    line,
                    col,
                });
                i = after_pos;
                continue 'outer;
            }
        }

        i += 1;
    }

    diags
}
