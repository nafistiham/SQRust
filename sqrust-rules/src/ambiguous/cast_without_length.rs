use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CastWithoutLength;

impl Rule for CastWithoutLength {
    fn name(&self) -> &'static str {
        "Ambiguous/CastWithoutLength"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

// ── source-level scan ─────────────────────────────────────────────────────────

/// Type names to detect (paired with their display name for the message).
const TYPES: &[&str] = &["NVARCHAR", "NCHAR", "VARCHAR", "CHAR"];

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // Pattern: `AS <TYPE>)` where <TYPE> is not followed by `(`.
    // We look for `AS ` (with optional spaces), then the type keyword, then `)`.
    let as_kw = b"AS";
    let as_len = as_kw.len();

    let mut i = 0;
    while i + as_len <= len {
        // Skip positions inside string literals or comments.
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Check word boundary before `AS`.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match of `AS`.
        if !bytes[i..i + as_len].eq_ignore_ascii_case(as_kw) {
            i += 1;
            continue;
        }

        // Ensure none of `AS` bytes are in string/comment.
        let all_code = (0..as_len).all(|k| !skip.contains(&(i + k)));
        if !all_code {
            i += 1;
            continue;
        }

        let as_end = i + as_len;

        // Word boundary after `AS`.
        let after_as_ok = as_end >= len || !is_word_char(bytes[as_end]);
        if !after_as_ok {
            i += 1;
            continue;
        }

        // Skip whitespace after `AS`.
        let mut j = as_end;
        while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }

        // Try to match one of our type keywords.
        if let Some(type_name) = match_type_keyword(bytes, len, j, &skip) {
            let type_end = j + type_name.len();

            // Skip any whitespace between the type name and the next non-space char.
            let mut k = type_end;
            while k < len && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }

            // If the next real character is `(`, the type has a length — not a violation.
            if k < len && bytes[k] == b'(' {
                i += 1;
                continue;
            }

            // If the next real character is `)`, it is `CAST(... AS TYPE)` — violation.
            if k < len && bytes[k] == b')' {
                let (line, col) = line_col(source, i);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: format!(
                        "CAST to {type_name} without length is implementation-defined; \
                         use {type_name}(N) with an explicit length",
                        type_name = type_name,
                    ),
                    line,
                    col,
                });
                i = k + 1;
                continue;
            }
        }

        i += 1;
    }

    diags
}

/// Attempts to match one of the tracked type keywords at position `pos`.
/// Returns `Some(&str)` with the canonical (uppercase) type name on success,
/// `None` otherwise.
fn match_type_keyword(
    bytes: &[u8],
    len: usize,
    pos: usize,
    skip: &std::collections::HashSet<usize>,
) -> Option<&'static str> {
    for &type_name in TYPES {
        let kw = type_name.as_bytes();
        let kw_len = kw.len();
        if pos + kw_len > len {
            continue;
        }
        // Case-insensitive match.
        if !bytes[pos..pos + kw_len].eq_ignore_ascii_case(kw) {
            continue;
        }
        // Ensure none of these bytes are inside string/comment.
        let all_code = (0..kw_len).all(|k| !skip.contains(&(pos + k)));
        if !all_code {
            continue;
        }
        // Word boundary after the type keyword.
        let after = pos + kw_len;
        let after_ok = after >= len || !is_word_char(bytes[after]);
        if !after_ok {
            continue;
        }
        return Some(type_name);
    }
    None
}

// ── skip set (string literals and line comments) ──────────────────────────────

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

// ── helpers ───────────────────────────────────────────────────────────────────

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
