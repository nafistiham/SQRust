use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct WhenOnNewLine;

impl Rule for WhenOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/WhenOnNewLine"
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
    let mut diags = Vec::new();

    let mut line_start_offset = 0usize;
    for (line_idx, line) in source.split('\n').enumerate() {
        let line_num = line_idx + 1;
        let line_upper = line.to_ascii_uppercase();
        let line_bytes = line_upper.as_bytes();
        let line_len = line_bytes.len();

        let mut search_from = 0usize;
        while search_from + 4 <= line_len {
            if let Some(pos) = find_word_in(line_bytes, b"WHEN", search_from) {
                let abs_offset = line_start_offset + pos;
                // Skip if inside a string/comment.
                if abs_offset < source.len() && !skip.is_code(abs_offset) {
                    search_from = pos + 4;
                    continue;
                }

                // Check whether there is non-whitespace content before WHEN on this line.
                let before_on_line = &line[..pos];
                if before_on_line.trim_start().is_empty() {
                    // Leading whitespace only — no violation.
                } else {
                    // col is 1-indexed, relative to the start of the line.
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "WHEN clause should start on a new line for readability"
                            .to_string(),
                        line: line_num,
                        col: pos + 1,
                    });
                }

                search_from = pos + 4;
            } else {
                break;
            }
        }

        // Advance past the line content + the '\n' separator.
        line_start_offset += line.len() + 1;
    }

    diags
}

/// Returns `true` if `source` (uppercased comparison) contains `keyword` as a
/// whole word.  `keyword` must already be uppercased.
fn contains_whole_word_upper(source: &str, keyword: &str) -> bool {
    let upper = source.to_ascii_uppercase();
    let haystack = upper.as_bytes();
    let needle = keyword.as_bytes();
    find_word_in(haystack, needle, 0).is_some()
}

/// Find the byte offset of `needle` as a whole word inside `haystack` starting
/// at `from`.  Returns `None` if not found.
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
