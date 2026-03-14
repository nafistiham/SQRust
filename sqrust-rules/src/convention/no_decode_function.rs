use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoDecodeFunction;

const MESSAGE: &str =
    "DECODE() is an Oracle-specific function; use CASE WHEN ... THEN ... END instead";

impl Rule for NoDecodeFunction {
    fn name(&self) -> &'static str {
        "Convention/NoDecodeFunction"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn build_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
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

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
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
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // "DECODE" is 6 characters
    let keyword = b"DECODE";
    let kw_len = keyword.len();

    let mut i = 0;
    while i + kw_len <= len {
        // Skip positions inside string literals or comments
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Check word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match of "DECODE"
        if !bytes[i..i + kw_len].eq_ignore_ascii_case(keyword) {
            i += 1;
            continue;
        }

        // Ensure none of the keyword bytes are in string/comment
        let all_code = (0..kw_len).all(|k| !skip.contains(&(i + k)));
        if !all_code {
            i += 1;
            continue;
        }

        let kw_end = i + kw_len;

        // Must be immediately followed by '(' to be a function call
        if kw_end >= len || bytes[kw_end] != b'(' {
            i += 1;
            continue;
        }

        let (line, col) = line_col(source, i);
        diags.push(Diagnostic {
            rule: rule_name,
            message: MESSAGE.to_string(),
            line,
            col,
        });

        i = kw_end + 1;
    }

    diags
}
