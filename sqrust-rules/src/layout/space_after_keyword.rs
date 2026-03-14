use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SpaceAfterKeyword;

/// SQL reserved keywords that must be followed by a space before `(`.
/// Function-like names (COALESCE, SUM, COUNT, etc.) are NOT in this list.
static KEYWORDS: &[&[u8]] = &[
    b"WHERE",
    b"AND",
    b"OR",
    b"NOT",
    b"IN",
    b"HAVING",
    b"ON",
    b"BETWEEN",
    b"CASE",
    b"WHEN",
    b"THEN",
    b"ELSE",
    b"EXISTS",
];

impl Rule for SpaceAfterKeyword {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterKeyword"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();
    let mut line: usize = 1;
    let mut line_start: usize = 0;
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
            i += 1;
            continue;
        }

        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match any keyword at position i.
        if let Some(kw) = match_keyword_at(bytes, len, &skip, i) {
            let after = i + kw.len();
            // Check: keyword immediately followed by '(' with no space.
            if after < len && bytes[after] == b'(' && !skip[after] {
                let col = i - line_start + 1;
                let kw_str = std::str::from_utf8(kw).unwrap_or("?");
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: format!(
                        "Keyword '{kw_str}' should be followed by a space before '(' \
                         — write '{kw_str} (' not '{kw_str}('"
                    ),
                    line,
                    col,
                });
                i += kw.len();
                continue;
            }
        }

        i += 1;
    }

    diags
}

/// If the bytes at `pos` match one of the reserved keywords (case-insensitive),
/// with a word boundary before and after, return the keyword slice.
fn match_keyword_at<'a>(
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    pos: usize,
) -> Option<&'a [u8]> {
    // Word boundary before: pos must not be preceded by an alphanumeric/underscore.
    let before_ok = pos == 0 || !is_word_char(bytes[pos - 1]);
    if !before_ok {
        return None;
    }

    for &kw in KEYWORDS {
        let kw_len = kw.len();
        if pos + kw_len > len {
            continue;
        }
        if !bytes[pos..pos + kw_len].eq_ignore_ascii_case(kw) {
            continue;
        }
        // Word boundary after: char right after keyword must not be alphanumeric/underscore.
        let after = pos + kw_len;
        let after_ok = after >= len || !is_word_char(bytes[after]);
        if after_ok && (after >= len || !skip[after]) {
            // Return the static slice from KEYWORDS so lifetime is 'a = 'static.
            return Some(kw);
        }
    }

    None
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
