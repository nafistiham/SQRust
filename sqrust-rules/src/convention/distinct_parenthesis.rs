use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct DistinctParenthesis;

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: each entry is `true` if the byte at that offset is
/// inside a string literal, line comment, block comment, or quoted identifier.
fn build_skip(source: &[u8]) -> Vec<bool> {
    let len = source.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && source[i] == b'-' && source[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && source[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && source[i] == b'/' && source[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                if i + 1 < len && source[i] == b'*' && source[i + 1] == b'/' {
                    skip[i] = true;
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote
        if source[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if source[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    if i < len && source[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if source[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len && source[i] != b'"' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if source[i] == b'`' {
            skip[i] = true;
            i += 1;
            while i < len && source[i] != b'`' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

impl Rule for DistinctParenthesis {
    fn name(&self) -> &'static str {
        "Convention/DistinctParenthesis"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        const DISTINCT: &[u8] = b"DISTINCT";
        const DISTINCT_LEN: usize = 8;

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip positions that are inside strings/comments
            if skip[i] {
                i += 1;
                continue;
            }

            // Look for the start of a word token matching DISTINCT (case-insensitive)
            if is_word_char(bytes[i]) {
                // Word boundary: not preceded by a word character
                let preceded_by_word = i > 0 && is_word_char(bytes[i - 1]);
                if preceded_by_word {
                    i += 1;
                    continue;
                }

                // Find the end of this word
                let word_start = i;
                let mut word_end = i;
                while word_end < len && is_word_char(bytes[word_end]) {
                    word_end += 1;
                }

                // Must be exactly 8 chars long and all code (not inside skip)
                if word_end - word_start == DISTINCT_LEN
                    && (word_start..word_end).all(|k| !skip[k])
                {
                    // Case-insensitive match against "DISTINCT"
                    let matches_distinct = bytes[word_start..word_end]
                        .iter()
                        .zip(DISTINCT.iter())
                        .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

                    if matches_distinct {
                        // Check what precedes DISTINCT (skip backwards over whitespace)
                        // If preceded by '(' it's inside a function like COUNT(DISTINCT ...)
                        // — that's not a violation.
                        let mut back = word_start;
                        while back > 0 && bytes[back - 1].is_ascii_whitespace() {
                            back -= 1;
                        }
                        let preceded_by_open_paren = back > 0 && bytes[back - 1] == b'(';

                        if !preceded_by_open_paren {
                            // Now look past DISTINCT for '('
                            let mut j = word_end;
                            // Skip whitespace after DISTINCT
                            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                                j += 1;
                            }
                            // If the next code character is '(', it's a violation
                            if j < len && !skip[j] && bytes[j] == b'(' {
                                let (line, col) = line_col(source, j);
                                diags.push(Diagnostic {
                                    rule: self.name(),
                                    message: "DISTINCT is not a function; write DISTINCT col instead of DISTINCT(col)".to_string(),
                                    line,
                                    col,
                                });
                            }
                        }

                        i = word_end;
                        continue;
                    }
                }

                i = word_end;
                continue;
            }

            i += 1;
        }

        diags
    }

    fn fix(&self, _ctx: &FileContext) -> Option<String> {
        // Fix is complex due to matching the closing parenthesis correctly.
        // Return None — flag only.
        None
    }
}
