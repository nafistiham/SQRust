use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoMinusOperator;

const MESSAGE: &str =
    "MINUS is Oracle-specific; use EXCEPT for standard SQL set difference";

impl Rule for NoMinusOperator {
    fn name(&self) -> &'static str {
        "Convention/NoMinusOperator"
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

/// Returns true if the trimmed content of the line starting at `line_start`
/// matches `MINUS` or `MINUS ALL` (case-insensitive), and nothing else.
/// `match_end` is the byte offset just after the matched "MINUS" keyword.
fn is_minus_set_operator_line(source: &str, line_start: usize, match_end: usize) -> bool {
    let bytes = source.as_bytes();
    let len = bytes.len();

    // Check that everything before the match on this line is whitespace
    let before_match = &source[line_start..match_end - 5]; // -5 for "MINUS"
    if !before_match.bytes().all(|b| b == b' ' || b == b'\t') {
        return false;
    }

    // After "MINUS", the rest of the line must be empty, or " ALL" (with optional trailing spaces)
    let mut j = match_end;
    // Skip spaces/tabs
    while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }

    // Either end of line/file, or "ALL" followed by end of line/file
    if j >= len || bytes[j] == b'\n' || bytes[j] == b'\r' {
        return true;
    }

    // Check for optional "ALL"
    let remaining = &source[j..];
    if remaining.len() >= 3 && remaining[..3].eq_ignore_ascii_case("ALL") {
        let after_all = j + 3;
        // After ALL must be end of line/file or optional whitespace then end of line
        let mut k = after_all;
        while k < len && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        return k >= len || bytes[k] == b'\n' || bytes[k] == b'\r';
    }

    false
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // "MINUS" is 5 characters
    let keyword = b"MINUS";
    let kw_len = keyword.len();

    // Pre-compute line start positions for each byte offset
    // We track the start of the current line as we scan
    let mut line_start = 0usize;
    let mut i = 0;

    while i + kw_len <= len {
        // Track line starts
        if i > 0 && bytes[i - 1] == b'\n' {
            line_start = i;
        }

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

        // Case-insensitive match of "MINUS"
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

        // Check word boundary after
        if kw_end < len && is_word_char(bytes[kw_end]) {
            i += 1;
            continue;
        }

        // Only flag if MINUS appears as the set operator: the trimmed line
        // must be exactly "MINUS" or "MINUS ALL" and nothing else.
        if is_minus_set_operator_line(source, line_start, kw_end) {
            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: MESSAGE.to_string(),
                line,
                col,
            });
        }

        i = kw_end;
    }

    diags
}
