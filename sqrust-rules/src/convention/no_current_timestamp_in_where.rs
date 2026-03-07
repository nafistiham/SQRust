use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoCurrentTimestampInWhere;

/// Non-deterministic timestamp functions to flag when used in WHERE/HAVING/ON.
/// We match these case-insensitively.  CURRENT_TIMESTAMP can appear without
/// parentheses; the others always have `(`.
///
/// We do NOT flag CURRENT_DATE because it is deterministic per-query execution
/// (the date does not change within a single query).
const TIMESTAMP_FUNCTIONS: &[&str] = &["CURRENT_TIMESTAMP", "NOW", "GETDATE", "SYSDATE"];

impl Rule for NoCurrentTimestampInWhere {
    fn name(&self) -> &'static str {
        "Convention/NoCurrentTimestampInWhere"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let mut diags = Vec::new();

        // Text-based scan: iterate over lines.
        // For each line that contains a WHERE/HAVING/ON clause keyword
        // (word-boundary, case-insensitive) and also contains one of the
        // timestamp function names, emit a violation.
        // We skip lines that are entirely inside `--` comments.
        // We also skip occurrences inside string literals (`'...'`).

        let mut line_num: usize = 0;
        let mut in_block_comment = false;

        for raw_line in source.lines() {
            line_num += 1;

            // Handle block comment state: track `/*` and `*/` across lines.
            // We update `in_block_comment` as we scan the line.
            let line_to_check = strip_non_code(raw_line, &mut in_block_comment);

            // Skip pure comment lines (after stripping, nothing left).
            if line_to_check.trim().is_empty() {
                continue;
            }

            // Check if the stripped line contains WHERE/HAVING/ON (word-boundary).
            let upper = line_to_check.to_uppercase();
            let has_clause = contains_word(&upper, "WHERE")
                || contains_word(&upper, "HAVING")
                || contains_word(&upper, "ON");

            if !has_clause {
                continue;
            }

            // Check if the stripped line also contains a timestamp function.
            for &func in TIMESTAMP_FUNCTIONS {
                if contains_word(&upper, func) {
                    // Find the column of the first occurrence of the function.
                    let col = find_word_col(&line_to_check, func);
                    diags.push(Diagnostic {
                        rule: "Convention/NoCurrentTimestampInWhere",
                        message: format!(
                            "Avoid CURRENT_TIMESTAMP/NOW() in WHERE/HAVING/JOIN; results may be non-deterministic"
                        ),
                        line: line_num,
                        col,
                    });
                    // Report once per line even if multiple functions appear.
                    break;
                }
            }
        }

        diags
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Strips everything inside `--` line comments and `'...'` string literals
/// from a single source line, taking block-comment state into account.
/// Updates `*in_block_comment` in place.
///
/// Returns the stripped version of the line (code portions only).
fn strip_non_code(line: &str, in_block_comment: &mut bool) -> String {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut result = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        if *in_block_comment {
            // Look for `*/` to end the block comment.
            if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                *in_block_comment = false;
                i += 2;
            } else {
                // Replace comment char with space to preserve column positions.
                result.push(b' ');
                i += 1;
            }
            continue;
        }

        // Line comment: `--` — rest of line is comment.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            break;
        }

        // Block comment start: `/*`.
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            *in_block_comment = true;
            result.push(b' ');
            result.push(b' ');
            i += 2;
            continue;
        }

        // Single-quoted string: skip contents.
        if bytes[i] == b'\'' {
            result.push(b'\'');
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    result.push(b'\'');
                    i += 1;
                    // `''` is an escaped quote inside the string.
                    if i < len && bytes[i] == b'\'' {
                        result.push(b'\'');
                        i += 1;
                        continue;
                    }
                    break;
                }
                // Replace string content with space to preserve positions.
                result.push(b' ');
                i += 1;
            }
            continue;
        }

        result.push(bytes[i]);
        i += 1;
    }

    // Safety: we only push ASCII bytes or valid UTF-8.
    String::from_utf8(result).unwrap_or_default()
}

/// Returns `true` if `haystack` contains `word` as a whole word
/// (case-sensitive — caller should upper-case both before calling).
fn contains_word(haystack: &str, word: &str) -> bool {
    let h = haystack.as_bytes();
    let w = word.as_bytes();
    let h_len = h.len();
    let w_len = w.len();

    if w_len == 0 || h_len < w_len {
        return false;
    }

    let mut i = 0;
    while i + w_len <= h_len {
        if h[i..i + w_len] == *w {
            let before_ok = i == 0 || !is_word_char(h[i - 1]);
            let after = i + w_len;
            let after_ok = after >= h_len || !is_word_char(h[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }

    false
}

/// Returns the 1-indexed column of the first whole-word occurrence of `word`
/// in `line` (case-insensitive).  Falls back to 1.
fn find_word_col(line: &str, word: &str) -> usize {
    let line_upper = line.to_uppercase();
    let h = line_upper.as_bytes();
    let w = word.as_bytes();
    let h_len = h.len();
    let w_len = w.len();

    if w_len == 0 {
        return 1;
    }

    let mut i = 0;
    while i + w_len <= h_len {
        if h[i..i + w_len] == *w {
            let before_ok = i == 0 || !is_word_char(h[i - 1]);
            let after = i + w_len;
            let after_ok = after >= h_len || !is_word_char(h[after]);
            if before_ok && after_ok {
                return i + 1; // 1-indexed
            }
        }
        i += 1;
    }

    1
}

fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
