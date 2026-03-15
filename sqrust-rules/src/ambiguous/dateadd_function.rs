use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct DateaddFunction;

/// Function specs: (uppercase name, message).
const FUNCTIONS: &[(&str, &str)] = &[
    (
        "DATEADD",
        "DATEADD() is SQL Server/Sybase-specific; use standard interval arithmetic (date + INTERVAL n unit) for portable SQL",
    ),
    (
        "DATE_ADD",
        "DATE_ADD() is MySQL-specific; use standard interval arithmetic (date + INTERVAL n unit) for portable SQL",
    ),
];

impl Rule for DateaddFunction {
    fn name(&self) -> &'static str {
        "Ambiguous/DateaddFunction"
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

    for (func_name, message) in FUNCTIONS {
        scan_for_function(source, bytes, len, &skip, func_name, message, rule_name, &mut diags);
    }

    diags.sort_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));
    diags
}

/// Scan for `func_name(` (case-insensitive) with word boundaries on both sides.
fn scan_for_function(
    source: &str,
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    func_name: &str,
    message: &str,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    let kw = func_name.as_bytes();
    let kw_len = kw.len();
    let mut i = 0;

    while i + kw_len <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            let after = i + kw_len;
            // Word boundary after: next char must not be a word char
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                // Skip optional whitespace then check for '('
                let mut j = after;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < len && bytes[j] == b'(' {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: message.to_string(),
                        line,
                        col,
                    });
                    i += kw_len;
                    continue;
                }
            }
        }

        i += 1;
    }
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
