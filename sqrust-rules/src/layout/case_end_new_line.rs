use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct CaseEndNewLine;

impl Rule for CaseEndNewLine {
    fn name(&self) -> &'static str {
        "Layout/CaseEndNewLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    // Only check multi-line queries.
    if !source.contains('\n') {
        return Vec::new();
    }

    // Only check SQL that contains a CASE expression.
    if !contains_whole_word_upper(source, "CASE") {
        return Vec::new();
    }

    let skip = SkipMap::build(source);
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let upper_source = source.to_ascii_uppercase();
    let upper_bytes = upper_source.as_bytes();

    let mut i = 0usize;
    while i + 3 <= len {
        // Look for END as a whole word in the uppercased source.
        if &upper_bytes[i..i + 3] == b"END" {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after = i + 3;
            let after_ok = after >= len || !is_word_char(bytes[after]);

            if before_ok && after_ok && skip.is_code(i) {
                // Find the start of the line containing this END.
                let line_start = find_line_start(bytes, i);
                // Extract the content before END on this line.
                let before_slice = &source[line_start..i];
                if !before_slice.trim_start().is_empty() {
                    // There is non-whitespace content before END on this line.
                    let (line_num, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message:
                            "END keyword should be on its own line in multi-line CASE expressions"
                                .to_string(),
                        line: line_num,
                        col,
                    });
                }
            }
            i += 3;
            continue;
        }
        i += 1;
    }

    diags
}

/// Returns the byte offset of the start of the line that contains `offset`.
fn find_line_start(bytes: &[u8], offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let mut i = offset - 1;
    loop {
        if bytes[i] == b'\n' {
            return i + 1;
        }
        if i == 0 {
            return 0;
        }
        i -= 1;
    }
}

/// Convert a byte offset in `source` to (line, col), both 1-indexed.
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

/// Returns `true` if `source` contains `keyword` (uppercase) as a whole word.
fn contains_whole_word_upper(source: &str, keyword: &str) -> bool {
    let upper = source.to_ascii_uppercase();
    let haystack = upper.as_bytes();
    let needle = keyword.as_bytes();
    find_word_in(haystack, needle, 0).is_some()
}

/// Find the byte offset of `needle` as a whole word inside `haystack` starting
/// at `from`.
fn find_word_in(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let n_len = needle.len();
    let h_len = haystack.len();
    if n_len > h_len {
        return None;
    }
    let mut i = from;
    while i + n_len <= h_len {
        if &haystack[i..i + n_len] == needle {
            let before_ok = i == 0 || !is_word_char(haystack[i - 1]);
            let after = i + n_len;
            let after_ok = after >= h_len || !is_word_char(haystack[after]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
