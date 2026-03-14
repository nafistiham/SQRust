use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct DateArithmetic;

impl Rule for DateArithmetic {
    fn name(&self) -> &'static str {
        "Ambiguous/DateArithmetic"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        find_date_arithmetic_violations(source, ctx)
    }
}

/// Prefixes that indicate the identifier is a date/time column when it starts with them.
/// e.g. `date_col`, `timestamp_field`, `created_at`, `updated_on`, `ts_start`.
const DATE_PREFIXES: &[&str] = &[
    "date", "time", "timestamp", "ts", "created", "updated", "modified",
];

/// Suffixes (after an underscore) that indicate the identifier is a date/time column.
/// e.g. `order_date`, `event_time`, `created_at`, `updated_on`.
const DATE_SUFFIXES: &[&str] = &[
    "date", "time", "timestamp", "ts", "at", "on", "created", "updated", "modified",
];

/// Returns `true` if the identifier token looks like a date/time column.
/// Matches when the identifier starts with a date prefix (e.g. `date_col`, `ts_start`)
/// or ends with a date suffix after an underscore (e.g. `created_at`, `order_date`).
/// Does NOT match when the date hint appears only in the middle (e.g. `non_date_col`).
fn is_date_like_identifier(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();

    // Check prefixes: identifier starts with the prefix and is followed by _ or is the whole token.
    for prefix in DATE_PREFIXES {
        if lower == *prefix {
            return true;
        }
        let prefixed = format!("{}_", prefix);
        if lower.starts_with(&prefixed) {
            return true;
        }
    }

    // Check suffixes: identifier ends with _suffix.
    for suffix in DATE_SUFFIXES {
        let suffixed = format!("_{}", suffix);
        if lower.ends_with(&suffixed) {
            return true;
        }
    }

    false
}

/// Returns `true` if the token is a plain (non-negative) integer literal.
fn is_integer_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit())
}

/// Returns `true` if `b` is a valid identifier character (alphanumeric or underscore).
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Scans `source` skipping string literals and comments, finding tokens around
/// `+` and `-` operators. Flags patterns where one side is a date-hint identifier
/// and the other side is a plain integer literal.
fn find_date_arithmetic_violations(source: &str, _ctx: &FileContext) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip string literals delimited by single quotes.
        if bytes[i] == b'\'' {
            i += 1;
            while i < len && bytes[i] != b'\'' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1; // skip closing quote
            continue;
        }

        // Skip line comments (-- to end of line).
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip block comments (/* ... */).
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2; // skip closing */
            continue;
        }

        // Check for + or - operator.
        if bytes[i] == b'+' || bytes[i] == b'-' {
            let op_pos = i;

            // Look backwards for the left token (skip whitespace).
            let left_end = scan_back_skip_whitespace(bytes, op_pos);
            let left_token = extract_token_backwards(bytes, left_end);

            // Look forwards for the right token (skip whitespace).
            let right_start = scan_forward_skip_whitespace(bytes, op_pos + 1);
            let right_token = extract_token_forwards(bytes, right_start);

            // Flag if (date_like_identifier op integer) or (integer op date_like_identifier).
            let should_flag = (!left_token.is_empty()
                && !right_token.is_empty()
                && is_date_like_identifier(&left_token)
                && is_integer_token(&right_token))
                || (!left_token.is_empty()
                    && !right_token.is_empty()
                    && is_integer_token(&left_token)
                    && is_date_like_identifier(&right_token));

            if should_flag {
                let (line, col) = offset_to_line_col(source, op_pos);
                diags.push(Diagnostic {
                    rule: "Ambiguous/DateArithmetic",
                    message: "Date arithmetic with integer offset is database-specific \
                               — use INTERVAL '1' DAY or dialect-specific functions for portability"
                        .to_string(),
                    line,
                    col,
                });
            }
        }

        i += 1;
    }

    diags
}

/// Returns the index of the last non-whitespace byte before `pos` (exclusive).
/// Returns `pos` itself if nothing meaningful is before it.
fn scan_back_skip_whitespace(bytes: &[u8], pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut j = pos - 1;
    while j > 0 && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
        j -= 1;
    }
    j
}

/// Returns the index of the first non-whitespace byte at or after `pos`.
fn scan_forward_skip_whitespace(bytes: &[u8], pos: usize) -> usize {
    let mut j = pos;
    while j < bytes.len()
        && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
    {
        j += 1;
    }
    j
}

/// Extracts the identifier or integer token ending at byte index `end_pos` (inclusive).
/// Works backwards from `end_pos` to find the token start.
fn extract_token_backwards(bytes: &[u8], end_pos: usize) -> String {
    if end_pos >= bytes.len() || !is_ident_char(bytes[end_pos]) {
        return String::new();
    }
    let mut start = end_pos;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }
    String::from_utf8_lossy(&bytes[start..=end_pos]).into_owned()
}

/// Extracts the identifier or integer token starting at byte index `start_pos`.
fn extract_token_forwards(bytes: &[u8], start_pos: usize) -> String {
    if start_pos >= bytes.len() || !is_ident_char(bytes[start_pos]) {
        return String::new();
    }
    let mut end = start_pos;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }
    String::from_utf8_lossy(&bytes[start_pos..end]).into_owned()
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
