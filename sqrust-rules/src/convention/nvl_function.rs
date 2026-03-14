use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NvlFunction;

const MESSAGE_NVL: &str =
    "NVL() is Oracle-specific; use COALESCE() for standard SQL";

const MESSAGE_NVL2: &str =
    "NVL2() is Oracle-specific; use CASE WHEN col IS NOT NULL THEN ... ELSE ... END instead";

impl Rule for NvlFunction {
    fn name(&self) -> &'static str {
        "Convention/NvlFunction"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn build_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // We search for both NVL2( (4 chars + 1 paren) and NVL( (3 chars + 1 paren).
    // Try NVL2 first at each position, then fall back to NVL.
    let nvl2 = b"NVL2";
    let nvl = b"NVL";

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Try to match NVL2 first (4 chars)
        if i + nvl2.len() <= len
            && bytes[i..i + nvl2.len()].eq_ignore_ascii_case(nvl2)
        {
            // Ensure all keyword bytes are code
            let all_code = (0..nvl2.len()).all(|k| !skip.contains(&(i + k)));
            if all_code {
                let kw_end = i + nvl2.len();
                // Must be followed by '(' and NOT followed by a word char (avoid NVL2X)
                let after_ok = kw_end < len
                    && bytes[kw_end] == b'('
                    && (kw_end + 1 >= len || !is_word_char(bytes[kw_end]));
                // The '(' itself already ensures it's not a longer word
                if kw_end < len && bytes[kw_end] == b'(' {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: MESSAGE_NVL2.to_string(),
                        line,
                        col,
                    });
                    i = kw_end + 1;
                    let _ = after_ok;
                    continue;
                }
            }
        }

        // Try to match NVL (3 chars), but ensure it's not NVL2 (word boundary after NVL)
        if i + nvl.len() <= len
            && bytes[i..i + nvl.len()].eq_ignore_ascii_case(nvl)
        {
            // Ensure all keyword bytes are code
            let all_code = (0..nvl.len()).all(|k| !skip.contains(&(i + k)));
            if all_code {
                let kw_end = i + nvl.len();
                // Must be immediately followed by '(' (not NVL2, NVLx, etc.)
                if kw_end < len && bytes[kw_end] == b'(' {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: MESSAGE_NVL.to_string(),
                        line,
                        col,
                    });
                    i = kw_end + 1;
                    continue;
                }
            }
        }

        i += 1;
    }

    diags
}
