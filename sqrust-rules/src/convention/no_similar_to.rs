use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct NoSimilarTo;

const MESSAGE: &str =
    "Avoid SIMILAR TO — use LIKE or a regex operator for portability";

impl Rule for NoSimilarTo {
    fn name(&self) -> &'static str {
        "Convention/NoSimilarTo"
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

/// Skip any whitespace at position `i` in `bytes`, returning the new position.
fn skip_whitespace(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    i
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if source.is_empty() {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let lower = source.to_lowercase();
    let bytes = lower.as_bytes();
    let len = bytes.len();

    let similar_kw = b"similar";
    let similar_len = similar_kw.len();
    let to_kw = b"to";
    let to_len = to_kw.len();

    let mut diags = Vec::new();
    let mut i = 0;

    while i + similar_len <= len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Check for "similar"
        if &bytes[i..i + similar_len] != similar_kw {
            i += 1;
            continue;
        }

        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        // Word boundary after "similar"
        let after_similar = i + similar_len;
        let after_similar_ok = after_similar >= len || !is_word_char(bytes[after_similar]);

        if !before_ok || !after_similar_ok {
            i += 1;
            continue;
        }

        // Skip whitespace between "similar" and "to"
        let to_start = skip_whitespace(bytes, after_similar);

        // Check there is at least one whitespace character (not immediately adjacent)
        if to_start == after_similar {
            // No whitespace — not SIMILAR TO
            i += 1;
            continue;
        }

        if to_start + to_len > len {
            i += 1;
            continue;
        }

        // Check "to" is not in skip set
        if skip.contains(&to_start) {
            i += 1;
            continue;
        }

        if &bytes[to_start..to_start + to_len] != to_kw {
            i += 1;
            continue;
        }

        // Word boundary after "to"
        let after_to = to_start + to_len;
        let after_to_ok = after_to >= len || !is_word_char(bytes[after_to]);

        if after_to_ok {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: MESSAGE.to_string(),
                line,
                col,
            });
            i = after_to;
        } else {
            i += 1;
        }
    }

    diags
}
