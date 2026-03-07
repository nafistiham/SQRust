use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct GrantAllPrivileges;

impl Rule for GrantAllPrivileges {
    fn name(&self) -> &'static str {
        "Lint/GrantAllPrivileges"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();
        let upper = source.to_uppercase();
        let upper_bytes = upper.as_bytes();

        let mut i = 0;
        while i < len {
            // Skip positions inside strings or comments.
            if skip[i] {
                i += 1;
                continue;
            }

            // Look for "GRANT" keyword (word-boundary, case-insensitive via upper).
            if !upper[i..].starts_with("GRANT") {
                i += 1;
                continue;
            }

            let grant_start = i;
            let grant_end = i + 5; // "GRANT".len()

            // Word-boundary check before GRANT.
            let before_ok = grant_start == 0 || {
                let b = upper_bytes[grant_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after GRANT.
            let after_ok = grant_end >= len || {
                let b = upper_bytes[grant_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace after GRANT (outside skip zones).
            let mut j = grant_end;
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Check if next word is ALL (word-boundary).
            if j + 3 <= len && &upper[j..j + 3] == "ALL" {
                let all_end = j + 3;
                let all_after_ok = all_end >= len || {
                    let b = upper_bytes[all_end];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if all_after_ok && !skip[j] {
                    let (line, col) = offset_to_line_col(source, grant_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message:
                            "GRANT ALL is overly permissive; specify only the required privileges"
                                .to_string(),
                        line,
                        col,
                    });
                    // Advance past this GRANT ALL to continue scanning.
                    i = all_end;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` for every byte offset inside a string literal,
/// line comment, or block comment.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
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

        // Single-quoted string: '...' with '' as escaped quote
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    // '' inside a string is an escaped quote — continue in string
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
                skip[i] = true;
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
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
