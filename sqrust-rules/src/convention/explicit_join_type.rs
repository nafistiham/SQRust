use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct ExplicitJoinType;

/// SQL join qualifier keywords that must precede JOIN for it to be explicit.
const JOIN_QUALIFIERS: &[&[u8]] = &[
    b"INNER",
    b"LEFT",
    b"RIGHT",
    b"FULL",
    b"OUTER",
    b"CROSS",
    b"NATURAL",
    b"LATERAL",
];

impl Rule for ExplicitJoinType {
    fn name(&self) -> &'static str {
        "Convention/ExplicitJoinType"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Only look at code positions (not strings/comments).
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Require a word boundary start.
            if is_word_char(bytes[i]) && (i == 0 || !is_word_char(bytes[i - 1])) {
                // Read the word.
                let word_start = i;
                let mut word_end = i;
                while word_end < len && is_word_char(bytes[word_end]) {
                    word_end += 1;
                }
                let word = &bytes[word_start..word_end];

                // Check if the word is exactly "JOIN" (case-insensitive).
                if word.eq_ignore_ascii_case(b"JOIN") {
                    // Look backward through whitespace (still in code positions) to find
                    // the preceding word.
                    let preceding = preceding_word(bytes, word_start, &skip);

                    let is_qualified = match preceding {
                        Some(w) => JOIN_QUALIFIERS.iter().any(|q| w.eq_ignore_ascii_case(q)),
                        None => false,
                    };

                    if !is_qualified {
                        let (line, col) = offset_to_line_col(source, word_start);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Bare JOIN defaults to INNER JOIN — use INNER JOIN explicitly for clarity".to_string(),
                            line,
                            col,
                        });
                    }
                }

                i = word_end;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Scans backward from `pos` through whitespace to find the preceding word.
/// Returns a byte slice of that word, or `None` if there is no preceding word
/// in code positions.
fn preceding_word<'a>(bytes: &'a [u8], pos: usize, skip: &SkipMap) -> Option<&'a [u8]> {
    if pos == 0 {
        return None;
    }

    // Walk backward past whitespace.
    let mut j = pos - 1;
    while j > 0 && bytes[j].is_ascii_whitespace() {
        j -= 1;
    }
    if bytes[j].is_ascii_whitespace() {
        // reached position 0 and it is still whitespace
        return None;
    }

    // Only consider code positions.
    if !skip.is_code(j) {
        return None;
    }

    // We are at the last character of the preceding word; verify it is a word char.
    if !is_word_char(bytes[j]) {
        return None;
    }

    let word_end = j + 1;

    // Walk backward to find the start of the word.
    while j > 0 && is_word_char(bytes[j - 1]) && skip.is_code(j - 1) {
        j -= 1;
    }
    let word_start = j;

    Some(&bytes[word_start..word_end])
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
