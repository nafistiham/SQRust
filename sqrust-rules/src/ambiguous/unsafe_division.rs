use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct UnsafeDivision;

const MESSAGE: &str =
    "Division without NULLIF guard may produce divide-by-zero errors; \
     consider using expr / NULLIF(denominator, 0)";

impl Rule for UnsafeDivision {
    fn name(&self) -> &'static str {
        "Ambiguous/UnsafeDivision"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();

    let mut i = 0;
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        if bytes[i] == b'/' {
            // Skip `/*` (block comment start — already handled by skip_set, but
            // we guard here too to avoid false positives at boundary edges)
            if i + 1 < len && bytes[i + 1] == b'*' {
                i += 1;
                continue;
            }

            // Skip `//` (integer division in some dialects)
            if i + 1 < len && bytes[i + 1] == b'/' {
                i += 2;
                continue;
            }

            // The character after `/` must be a space or an identifier start
            // (not `*` which means block comment).
            let next = if i + 1 < len { bytes[i + 1] } else { 0 };
            if next == b'*' {
                i += 1;
                continue;
            }

            // Now determine whether this is an unguarded division.
            // Skip whitespace after `/` to find the denominator token.
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }

            if j >= len {
                i += 1;
                continue;
            }

            // If the denominator starts with a digit (numeric literal) — safe.
            if bytes[j].is_ascii_digit() {
                i += 1;
                continue;
            }

            // Check if the denominator starts with `NULLIF` (case-insensitive).
            let nullif = b"NULLIF";
            if j + nullif.len() <= len && bytes[j..j + nullif.len()].eq_ignore_ascii_case(nullif) {
                // Confirm word boundary after NULLIF
                let after_nullif = j + nullif.len();
                let boundary_ok = after_nullif >= len || !is_word_char(bytes[after_nullif]);
                if boundary_ok {
                    i += 1;
                    continue;
                }
            }

            // Unguarded division — flag the `/` position.
            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: MESSAGE.to_string(),
                line,
                col,
            });
        }

        i += 1;
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
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
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
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

        i += 1;
    }

    skip
}
