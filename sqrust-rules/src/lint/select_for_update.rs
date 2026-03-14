use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SelectForUpdate;

impl Rule for SelectForUpdate {
    fn name(&self) -> &'static str {
        "Lint/SelectForUpdate"
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
    let upper = source.to_uppercase();
    let upper_bytes = upper.as_bytes();

    let mut diags = Vec::new();

    // Scan for locking hint patterns.
    let mut i = 0;
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match each locking pattern at position i.
        if let Some((msg, advance)) = match_locking_hint(upper_bytes, &skip, i, len) {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: msg.to_string(),
                line,
                col,
            });
            i += advance;
            continue;
        }

        i += 1;
    }

    diags
}

/// Attempts to match a locking hint at byte position `pos` in `upper_bytes`.
/// Returns (message, bytes_to_advance) on a match, or None.
fn match_locking_hint(
    upper_bytes: &[u8],
    _skip: &[bool],
    pos: usize,
    len: usize,
) -> Option<(&'static str, usize)> {
    // All patterns start with FOR or WITH — quick reject.
    if pos >= len {
        return None;
    }

    let ch = upper_bytes[pos];

    if ch == b'F' {
        // FOR UPDATE / FOR UPDATE OF ... / FOR NO KEY UPDATE / FOR SHARE
        if !keyword_matches(upper_bytes, pos, b"FOR", len) {
            return None;
        }
        let after_for = pos + 3;
        let rest_start = skip_whitespace(upper_bytes, after_for, len);

        // FOR NO KEY UPDATE (PostgreSQL)
        if keyword_matches(upper_bytes, rest_start, b"NO", len) {
            let after_no = rest_start + 2;
            let kw_start = skip_whitespace(upper_bytes, after_no, len);
            if keyword_matches(upper_bytes, kw_start, b"KEY", len) {
                let after_key = kw_start + 3;
                let upd_start = skip_whitespace(upper_bytes, after_key, len);
                if keyword_matches(upper_bytes, upd_start, b"UPDATE", len) {
                    let end = upd_start + 6;
                    return Some((
                        "SELECT FOR UPDATE is dialect-specific row locking — behavior varies across databases; consider application-level locking or explicit transaction management",
                        end - pos,
                    ));
                }
            }
        }

        // FOR UPDATE (and FOR UPDATE OF ...)
        if keyword_matches(upper_bytes, rest_start, b"UPDATE", len) {
            let end = rest_start + 6;
            return Some((
                "SELECT FOR UPDATE is dialect-specific row locking — behavior varies across databases; consider application-level locking or explicit transaction management",
                end - pos,
            ));
        }

        // FOR SHARE
        if keyword_matches(upper_bytes, rest_start, b"SHARE", len) {
            let end = rest_start + 5;
            return Some((
                "SELECT FOR SHARE is dialect-specific row locking — not supported in all databases",
                end - pos,
            ));
        }

        return None;
    }

    if ch == b'W' {
        // WITH (UPDLOCK) — SQL Server
        if !keyword_matches(upper_bytes, pos, b"WITH", len) {
            return None;
        }
        let after_with = pos + 4;
        let rest_start = skip_whitespace(upper_bytes, after_with, len);
        if rest_start >= len || upper_bytes[rest_start] != b'(' {
            return None;
        }
        let inner_start = skip_whitespace(upper_bytes, rest_start + 1, len);
        if keyword_matches(upper_bytes, inner_start, b"UPDLOCK", len) {
            let after_updlock = inner_start + 7;
            let close_start = skip_whitespace(upper_bytes, after_updlock, len);
            if close_start < len && upper_bytes[close_start] == b')' {
                let end = close_start + 1;
                return Some((
                    "WITH (UPDLOCK) is SQL Server-specific locking hint — use portable transaction isolation levels instead",
                    end - pos,
                ));
            }
        }
        return None;
    }

    None
}

/// Returns true if `keyword` appears at `pos` in `bytes` at a word boundary.
fn keyword_matches(bytes: &[u8], pos: usize, keyword: &[u8], len: usize) -> bool {
    let kw_len = keyword.len();
    if pos + kw_len > len {
        return false;
    }
    if &bytes[pos..pos + kw_len] != keyword {
        return false;
    }
    // Word boundary before
    let before_ok = pos == 0 || !is_word_char(bytes[pos - 1]);
    if !before_ok {
        return false;
    }
    // Word boundary after
    let after = pos + kw_len;
    after >= len || !is_word_char(bytes[after])
}

/// Advances `i` over ASCII whitespace bytes.
fn skip_whitespace(bytes: &[u8], mut i: usize, len: usize) -> usize {
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
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
