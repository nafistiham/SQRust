use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoNvl2;

const MESSAGE: &str =
    "NVL2() is Oracle-specific; use CASE WHEN x IS NOT NULL THEN y ELSE z END instead";

impl Rule for NoNvl2 {
    fn name(&self) -> &'static str {
        "Convention/NoNvl2"
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

    // "NVL2" is 4 characters
    let keyword = b"NVL2";
    let kw_len = keyword.len();

    let mut i = 0;
    while i + kw_len <= len {
        // Skip positions inside string literals or comments
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match of "NVL2"
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

        // Word boundary after keyword (ensures NVL2 not part of longer identifier)
        let after_ok = kw_end >= len || !is_word_char(bytes[kw_end]);
        if !after_ok {
            i += 1;
            continue;
        }

        // Must be followed (after optional whitespace) by '(' to be a function call
        let mut j = kw_end;
        while j < len && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= len || bytes[j] != b'(' {
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
