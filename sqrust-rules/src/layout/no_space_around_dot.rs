use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoSpaceAroundDot;

impl Rule for NoSpaceAroundDot {
    fn name(&self) -> &'static str {
        "Layout/NoSpaceAroundDot"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

/// Build a boolean skip-set: byte is `true` if it lies inside a
/// single-quoted string, block comment (`/* */`), or line comment (`--`).
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip[start..=i + 1].fill(true);
                        i += 2;
                        continue;
                    }
                    skip[start..=i].fill(true);
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[start..=i + 1].fill(true);
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            skip[start..i].fill(true);
            continue;
        }

        i += 1;
    }

    skip
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

fn byte_offset_to_line_col(bytes: &[u8], offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for i in 0..offset {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = offset - line_start + 1;
    (line, col)
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();

    for pos in 0..len {
        if bytes[pos] != b'.' || skip[pos] {
            continue;
        }

        // Determine what's immediately before and after the dot (ignoring
        // double-quoted identifiers — we do NOT skip those since we want to
        // catch `"schema" . "table"` as well, but the typical case is plain
        // identifiers).

        // Find the nearest non-space character before the dot.
        let before_char: Option<u8> = if pos == 0 {
            None
        } else {
            let mut j = pos;
            loop {
                if j == 0 {
                    break None;
                }
                j -= 1;
                if bytes[j] != b' ' {
                    break Some(bytes[j]);
                }
            }
        };

        // Find the nearest non-space character after the dot.
        let after_char: Option<u8> = {
            let mut j = pos + 1;
            loop {
                if j >= len {
                    break None;
                }
                if bytes[j] != b' ' {
                    break Some(bytes[j]);
                }
                j += 1;
            }
        };

        // Skip pure float literals: digit . digit (even with spaces around).
        let before_is_digit = before_char.map(is_digit).unwrap_or(false);
        let after_is_digit = after_char.map(is_digit).unwrap_or(false);
        if before_is_digit && after_is_digit {
            continue;
        }

        // Check for space immediately before the dot.
        let space_before = pos > 0 && bytes[pos - 1] == b' ';
        // Check for space immediately after the dot.
        let space_after = pos + 1 < len && bytes[pos + 1] == b' ';

        // Only flag if at least one adjacent side has a word char (or the
        // other side is a word char), so we don't flag lone dots in prose.
        let before_is_word = before_char.map(is_word_char).unwrap_or(false);
        let after_is_word = after_char.map(is_word_char).unwrap_or(false);

        if (space_before || space_after) && (before_is_word || after_is_word) {
            let (line, col) = byte_offset_to_line_col(bytes, pos);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Spaces around '.' in qualified name — use 'schema.table' not 'schema . table'".to_string(),
                line,
                col,
            });
        }
    }

    diags
}
