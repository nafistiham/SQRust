use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NullSafeEquality;

impl Rule for NullSafeEquality {
    fn name(&self) -> &'static str {
        "Ambiguous/NullSafeEquality"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        find_violations(source, self.name())
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

        // Check for <=> (null-safe equality operator, MySQL/MariaDB-specific)
        if i + 2 < len && bytes[i] == b'<' && bytes[i + 1] == b'=' && bytes[i + 2] == b'>' {
            if !skip[i] && !skip[i + 1] && !skip[i + 2] {
                let col = i - line_start + 1;
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "The <=> null-safe equality operator is MySQL/MariaDB-specific — use 'IS NOT DISTINCT FROM' (ANSI) or 'COALESCE(a, sentinel) = COALESCE(b, sentinel)' for portable null-safe comparison".to_string(),
                    line,
                    col,
                });
                i += 3;
                continue;
            }
        }

        // Check for IS NOT DISTINCT FROM (case-insensitive)
        if let Some(end) = match_is_not_distinct_from(bytes, &skip, i) {
            let col = i - line_start + 1;
            diags.push(Diagnostic {
                rule: rule_name,
                message: "IS NOT DISTINCT FROM has inconsistent support across databases — verify your target dialect supports it, or use COALESCE-based comparison".to_string(),
                line,
                col,
            });
            i = end;
            continue;
        }

        // Check for IS DISTINCT FROM (case-insensitive) — but not IS NOT DISTINCT FROM
        if let Some(end) = match_is_distinct_from(bytes, &skip, i) {
            let col = i - line_start + 1;
            diags.push(Diagnostic {
                rule: rule_name,
                message: "IS DISTINCT FROM has inconsistent support across databases — verify your target dialect supports it".to_string(),
                line,
                col,
            });
            i = end;
            continue;
        }

        i += 1;
    }

    diags
}

/// Match "IS NOT DISTINCT FROM" at position i, case-insensitive, with word boundaries.
/// Returns the offset just past the match if matched, or None.
fn match_is_not_distinct_from(bytes: &[u8], skip: &[bool], i: usize) -> Option<usize> {
    let len = bytes.len();

    // Word boundary before IS
    if i > 0 && is_word_char(bytes[i - 1]) {
        return None;
    }

    let after_is = match_keyword_at(bytes, skip, i, b"IS")?;
    let after_ws1 = skip_ws(bytes, after_is);
    if after_ws1 == after_is {
        return None; // must have whitespace
    }
    let after_not = match_keyword_at(bytes, skip, after_ws1, b"NOT")?;
    let after_ws2 = skip_ws(bytes, after_not);
    if after_ws2 == after_not {
        return None;
    }
    let after_distinct = match_keyword_at(bytes, skip, after_ws2, b"DISTINCT")?;
    let after_ws3 = skip_ws(bytes, after_distinct);
    if after_ws3 == after_distinct {
        return None;
    }
    let after_from = match_keyword_at(bytes, skip, after_ws3, b"FROM")?;

    // Ensure "FROM" is actually followed by a word boundary (i.e., not part of another word)
    // match_keyword_at already checks the end boundary, so this is handled.
    let _ = len;
    Some(after_from)
}

/// Match "IS DISTINCT FROM" at position i (but NOT IS NOT DISTINCT FROM).
/// Returns the offset just past the match if matched, or None.
fn match_is_distinct_from(bytes: &[u8], skip: &[bool], i: usize) -> Option<usize> {
    // Word boundary before IS
    if i > 0 && is_word_char(bytes[i - 1]) {
        return None;
    }

    let after_is = match_keyword_at(bytes, skip, i, b"IS")?;
    let after_ws1 = skip_ws(bytes, after_is);
    if after_ws1 == after_is {
        return None;
    }

    // Make sure next keyword is DISTINCT, not NOT
    // (IS NOT DISTINCT FROM is handled by match_is_not_distinct_from)
    if match_keyword_at(bytes, skip, after_ws1, b"NOT").is_some() {
        return None;
    }

    let after_distinct = match_keyword_at(bytes, skip, after_ws1, b"DISTINCT")?;
    let after_ws2 = skip_ws(bytes, after_distinct);
    if after_ws2 == after_distinct {
        return None;
    }
    let after_from = match_keyword_at(bytes, skip, after_ws2, b"FROM")?;

    Some(after_from)
}

/// Match a keyword (case-insensitive) at position i in bytes, with word-boundary checks.
/// Skips positions that are in the skip set.
/// Returns the offset just past the match, or None.
fn match_keyword_at(bytes: &[u8], skip: &[bool], i: usize, keyword: &[u8]) -> Option<usize> {
    let len = bytes.len();
    let klen = keyword.len();

    if i + klen > len {
        return None;
    }

    // Must start at a code position.
    if skip[i] {
        return None;
    }

    // Check case-insensitive match, every byte must be code.
    for k in 0..klen {
        if skip[i + k] {
            return None;
        }
        if !bytes[i + k].eq_ignore_ascii_case(&keyword[k]) {
            return None;
        }
    }

    // Word boundary after keyword.
    let end = i + klen;
    if end < len && is_word_char(bytes[end]) {
        return None;
    }

    Some(end)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Skip ASCII whitespace at position i.
fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len()
        && (bytes[i] == b' '
            || bytes[i] == b'\t'
            || bytes[i] == b'\n'
            || bytes[i] == b'\r')
    {
        i += 1;
    }
    i
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
