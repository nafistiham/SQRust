use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct LikeEscapeChar;

impl Rule for LikeEscapeChar {
    fn name(&self) -> &'static str {
        "Ambiguous/LikeEscapeChar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Look for LIKE or ILIKE keyword in code (not in strings/comments).
            // We need to find the keyword boundary so we don't match sub-words.
            if skip.is_code(i) {
                let (is_like, kw_len) = find_like_keyword(bytes, i, len, &skip);
                if is_like {
                    // Record where the LIKE keyword ends so we can search for the string literal.
                    let after_like = i + kw_len;

                    // Skip whitespace after the keyword.
                    let mut j = after_like;
                    while j < len && skip.is_code(j) && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                        j += 1;
                    }

                    // The next code character should be the opening quote of the pattern.
                    // The opening quote is marked skip; the byte before it is code.
                    if j < len && bytes[j] == b'\'' && (j == 0 || skip.is_code(j - 1)) {
                        // j is the opening quote of the LIKE pattern string.
                        let str_start = j;
                        let content_start = j + 1;
                        let mut k = content_start;
                        // Walk through skip bytes (inside the string).
                        while k < len && !skip.is_code(k) {
                            k += 1;
                        }
                        // The closing quote was at k - 1.
                        let content_end = if k > content_start { k - 1 } else { content_start };
                        let content = &bytes[content_start..content_end];

                        // Check if pattern contains backslash-escaped wildcard.
                        if has_backslash_escape(content) {
                            // Check if an ESCAPE clause follows within ~50 chars after closing quote.
                            let after_str = k;
                            let end = source[after_str..].char_indices().nth(50)
                                .map(|(off, _)| after_str + off)
                                .unwrap_or(len);
                            let look_ahead = &source[after_str..end];
                            if !has_escape_clause(look_ahead) {
                                let (line, col) = offset_to_line_col(source, str_start);
                                diags.push(Diagnostic {
                                    rule: self.name(),
                                    message: "LIKE pattern uses backslash escape (\\_ or \\%) which requires an ESCAPE clause in standard SQL; add ESCAPE '\\\\' or use dialect-specific syntax".to_string(),
                                    line,
                                    col,
                                });
                            }
                        }

                        // Advance past the string.
                        i = k;
                        continue;
                    }

                    i = after_like;
                    continue;
                }
            }
            i += 1;
        }

        diags
    }
}

/// Returns `(true, keyword_length)` if there is a LIKE or ILIKE keyword
/// starting at `pos` in code context (not inside a string/comment), with
/// word boundaries on both sides. Returns `(false, 0)` otherwise.
fn find_like_keyword(bytes: &[u8], pos: usize, len: usize, skip: &SkipMap) -> (bool, usize) {
    // Must be a code byte.
    if !skip.is_code(pos) {
        return (false, 0);
    }

    // Check for ILIKE (5 chars) first, then LIKE (4 chars).
    for &(keyword, klen) in &[(b"ILIKE" as &[u8], 5usize), (b"LIKE" as &[u8], 4usize)] {
        if pos + klen > len {
            continue;
        }
        // Case-insensitive compare.
        let matches = bytes[pos..pos + klen]
            .iter()
            .zip(keyword.iter())
            .all(|(&b, &k)| b.to_ascii_uppercase() == k);
        if !matches {
            continue;
        }
        // Left boundary: pos == 0 or previous byte is not a word char.
        let left_ok = pos == 0 || !is_word_char(bytes[pos - 1]);
        // Right boundary: next byte after keyword is not a word char (or end).
        let right_ok = pos + klen >= len || !is_word_char(bytes[pos + klen]);
        if left_ok && right_ok {
            return (true, klen);
        }
    }

    (false, 0)
}

/// Returns `true` if the byte slice contains `\_` or `\%`.
fn has_backslash_escape(content: &[u8]) -> bool {
    let mut i = 0;
    while i + 1 < content.len() {
        if content[i] == b'\\' && (content[i + 1] == b'_' || content[i + 1] == b'%') {
            return true;
        }
        i += 1;
    }
    false
}

/// Returns `true` if the look-ahead string starts with an ESCAPE keyword
/// (ignoring leading whitespace).
fn has_escape_clause(s: &str) -> bool {
    let trimmed = s.trim_start();
    let upper: String = trimmed.chars().take(6).collect::<String>().to_uppercase();
    upper.starts_with("ESCAPE")
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
