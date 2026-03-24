pub mod functions;
pub mod keywords;
pub mod literals;
pub mod types;

/// Tokenises a SQL source string into a sequence of `Token`s.
///
/// Tokens are either:
/// - `Code(start_byte)` — a single character of real SQL code, with its byte offset
/// - `Skip` — a character that is inside a string/comment/quoted-identifier and
///   should be ignored by rules
///
/// Rules iterate over the source and use `is_code_at` to decide whether a
/// character at a given byte offset should be inspected.
pub(crate) struct SkipMap {
    /// `true` for every byte offset that is inside a string, comment, or
    /// quoted identifier and must be skipped.
    skip: Vec<bool>,
}

impl SkipMap {
    pub(crate) fn build(source: &str) -> Self {
        let bytes = source.as_bytes();
        let len = source.len();
        let mut skip = vec![false; len];

        let mut i = 0;
        while i < len {
            // Line comment: -- ... end-of-line
            if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                skip[i] = true;
                skip[i + 1] = true;
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    skip[i] = true;
                    i += 1;
                }
                // '\n' itself is not skipped so line numbers stay correct
                continue;
            }

            // Block comment: /* ... */
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                skip[i] = true;
                skip[i + 1] = true;
                i += 2;
                while i < len {
                    if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
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

            // Single-quoted string: '...' with '' escape (SQL standard)
            if bytes[i] == b'\'' {
                skip[i] = true;
                i += 1;
                while i < len {
                    if bytes[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        // '' is an escaped quote inside the string, not the end
                        if i < len && bytes[i] == b'\'' {
                            skip[i] = true;
                            i += 1;
                            continue;
                        }
                        break; // end of string
                    }
                    skip[i] = true;
                    i += 1;
                }
                continue;
            }

            // Double-quoted identifier: "..."
            if bytes[i] == b'"' {
                skip[i] = true;
                i += 1;
                while i < len && bytes[i] != b'"' {
                    skip[i] = true;
                    i += 1;
                }
                if i < len {
                    skip[i] = true; // closing "
                    i += 1;
                }
                continue;
            }

            // Backtick identifier: `...`
            if bytes[i] == b'`' {
                skip[i] = true;
                i += 1;
                while i < len && bytes[i] != b'`' {
                    skip[i] = true;
                    i += 1;
                }
                if i < len {
                    skip[i] = true; // closing `
                    i += 1;
                }
                continue;
            }

            // Jinja/dbt template block: {{ ... }} and {% ... %} and {# ... #}
            if i + 1 < len && bytes[i] == b'{' && (bytes[i + 1] == b'{' || bytes[i + 1] == b'%' || bytes[i + 1] == b'#') {
                let closing_inner = match bytes[i + 1] {
                    b'{' => b'}',
                    b'%' => b'%',
                    b'#' => b'#',
                    _ => unreachable!(),
                };
                skip[i] = true;
                skip[i + 1] = true;
                i += 2;
                while i < len {
                    if bytes[i] == closing_inner && i + 1 < len && bytes[i + 1] == b'}' {
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

            i += 1;
        }

        SkipMap { skip }
    }

    /// Returns `true` if the byte at `offset` is real SQL code (not inside a
    /// string / comment / quoted identifier).
    #[inline]
    pub(crate) fn is_code(&self, offset: usize) -> bool {
        !self.skip[offset]
    }
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
pub(crate) fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
