use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct PreferCoalesceOverNullCase;

const MESSAGE: &str =
    "CASE WHEN x IS NULL THEN y ELSE x END can be simplified to COALESCE(x, y)";

impl Rule for PreferCoalesceOverNullCase {
    fn name(&self) -> &'static str {
        "Convention/PreferCoalesceOverNullCase"
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

/// Find the next occurrence of `keyword` (ASCII, case-insensitive) in `lower_bytes` starting at
/// position `start`, that is not in the skip set and respects word boundaries.
/// Returns the start offset of the match, or `None`.
fn find_keyword_from(
    lower_bytes: &[u8],
    keyword: &[u8],
    skip: &HashSet<usize>,
    start: usize,
) -> Option<usize> {
    let len = lower_bytes.len();
    let kw_len = keyword.len();
    let mut i = start;
    while i + kw_len <= len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }
        if &lower_bytes[i..i + kw_len] == keyword {
            let before_ok = i == 0 || !is_word_char(lower_bytes[i - 1]);
            let after_pos = i + kw_len;
            let after_ok = after_pos >= len || !is_word_char(lower_bytes[after_pos]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Find `CASE WHEN` as a two-keyword sequence. Returns the offset of `CASE` and the end of `WHEN`.
fn find_case_when_from(
    lower_bytes: &[u8],
    skip: &HashSet<usize>,
    start: usize,
) -> Option<(usize, usize)> {
    let len = lower_bytes.len();
    let case_kw = b"case";
    let when_kw = b"when";
    let case_len = case_kw.len();
    let when_len = when_kw.len();

    let mut i = start;
    while i + case_len <= len {
        // Find CASE
        let case_pos = find_keyword_from(lower_bytes, case_kw, skip, i)?;
        // After CASE, skip whitespace and look for WHEN
        let mut j = case_pos + case_len;
        while j < len && (lower_bytes[j] == b' ' || lower_bytes[j] == b'\t' || lower_bytes[j] == b'\n' || lower_bytes[j] == b'\r') {
            j += 1;
        }
        // WHEN must follow immediately (possibly after whitespace)
        if j + when_len <= len && &lower_bytes[j..j + when_len] == when_kw {
            let after_when = j + when_len;
            let after_ok = after_when >= len || !is_word_char(lower_bytes[after_when]);
            if after_ok {
                return Some((case_pos, after_when));
            }
        }
        i = case_pos + 1;
    }
    None
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    if source.is_empty() {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let lower = source.to_lowercase();
    let lower_bytes = lower.as_bytes();

    let is_null_kw = b"is null";
    let is_not_null_kw = b"is not null";
    let proximity_limit: usize = 500;

    let mut diags = Vec::new();
    let mut search_from = 0;

    loop {
        // Find the next CASE WHEN
        let Some((case_pos, after_when)) = find_case_when_from(lower_bytes, &skip, search_from)
        else {
            break;
        };

        // Look for IS NULL within proximity_limit chars after WHEN
        let window_end = (after_when + proximity_limit).min(lower_bytes.len());

        // Check there's no IS NOT NULL match (we want IS NULL but not IS NOT NULL)
        // We look for IS NULL first, then ensure it's not part of IS NOT NULL
        let is_null_pos =
            find_keyword_from(&lower_bytes[..window_end], is_null_kw, &skip, after_when);

        if let Some(null_pos) = is_null_pos {
            // Make sure it's not IS NOT NULL — check if "not" appears between "is" and "null"
            // The is_null_kw already matched exactly "is null" with word boundaries,
            // but "IS NOT NULL" would not match "is null" at the same position because
            // "is not null" != "is null". So if we found "is null", we need to verify
            // it isn't actually "is not null" (i.e., the bytes between IS and NULL contain NOT).
            // Since find_keyword_from matched "is null" literally, "IS NOT NULL" would be
            // detected as "is null" if only checking "is null" — the "not " sits between them.
            // However, "is null" (7 bytes) vs "is not null" (11 bytes): find finds "is null"
            // starting at "is" but in "IS NOT NULL" the bytes starting at "is" are "is not null"
            // which doesn't start with "is null". So a match of "is null" is always safe.
            // BUT: let's double-check by also trying to find is_not_null at same position.
            let also_is_not_null = find_keyword_from(
                &lower_bytes[..window_end],
                is_not_null_kw,
                &skip,
                after_when,
            )
            .map(|p| p == null_pos)
            .unwrap_or(false);

            if !also_is_not_null {
                let (line, col) = offset_to_line_col(source, case_pos);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: MESSAGE.to_string(),
                    line,
                    col,
                });
                // Advance past this CASE WHEN to find the next one
                search_from = null_pos + is_null_kw.len();
            } else {
                search_from = case_pos + 1;
            }
        } else {
            search_from = case_pos + 1;
        }
    }

    diags
}
