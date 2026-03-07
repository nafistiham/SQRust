use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct SelectTopN;

impl Rule for SelectTopN {
    fn name(&self) -> &'static str {
        "Convention/SelectTopN"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip non-code bytes (strings, comments).
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match SELECT at position i.
            if !matches_keyword_ci(bytes, len, &skip_map, i, b"SELECT") {
                i += 1;
                continue;
            }

            // Word boundary before SELECT.
            if i > 0 && is_word_char(bytes[i - 1]) {
                i += 1;
                continue;
            }

            let select_end = i + 6; // len("SELECT")

            // Word boundary after SELECT.
            if select_end < len && is_word_char(bytes[select_end]) {
                i += 1;
                continue;
            }

            // Advance past SELECT, skip whitespace to find TOP.
            // We must not cross a FROM keyword — that would mean TOP is in a
            // different clause. Scan forward consuming only whitespace and code.
            let mut j = select_end;

            // Skip whitespace after SELECT.
            while j < len && skip_map.is_code(j) && is_whitespace(bytes[j]) {
                j += 1;
            }

            // If next token is FROM, there is no TOP in this SELECT header.
            // Also abort if we hit a non-code byte (string start) before TOP.
            if j >= len || !skip_map.is_code(j) {
                i += 1;
                continue;
            }

            // We allow TOP to appear as the immediate next keyword after SELECT.
            // In T-SQL: SELECT TOP N [DISTINCT] …
            // Check for TOP at position j with word boundaries.
            if matches_keyword_ci(bytes, len, &skip_map, j, b"TOP")
                && (j == 0 || !is_word_char(bytes[j - 1]))
            {
                let top_end = j + 3; // len("TOP")
                let word_boundary_after =
                    top_end >= len || !is_word_char(bytes[top_end]);

                if word_boundary_after {
                    // Verify that TOP is followed by a numeric literal or '?'.
                    let mut k = top_end;
                    while k < len && skip_map.is_code(k) && is_whitespace(bytes[k]) {
                        k += 1;
                    }
                    let followed_by_number = k < len
                        && skip_map.is_code(k)
                        && (bytes[k].is_ascii_digit() || bytes[k] == b'?');

                    if followed_by_number {
                        let (line, col) = line_col(source, j);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Use LIMIT instead of TOP N for portability".to_string(),
                            line,
                            col,
                        });
                        i = k + 1;
                        continue;
                    }
                }
            }

            i += 1;
        }

        diags
    }
}

/// Returns true if bytes[pos..pos+kw.len()] matches `kw` case-insensitively and
/// every byte of that slice is code (not inside string/comment).
fn matches_keyword_ci(
    bytes: &[u8],
    len: usize,
    skip_map: &SkipMap,
    pos: usize,
    kw: &[u8],
) -> bool {
    let kw_len = kw.len();
    if pos + kw_len > len {
        return false;
    }
    (0..kw_len)
        .all(|k| skip_map.is_code(pos + k) && bytes[pos + k].eq_ignore_ascii_case(&kw[k]))
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[inline]
fn is_whitespace(ch: u8) -> bool {
    ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
