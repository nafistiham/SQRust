use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct RegexpFunction;

/// Regexp function names that are dialect-specific (matched as function calls with `(`).
const REGEXP_FUNCTIONS: &[&str] = &[
    "REGEXP_LIKE",
    "REGEXP_CONTAINS",
    "REGEXP_EXTRACT",
    "REGEXP_MATCH",
    "REGEXP_MATCHES",
    "REGEXP_SUBSTR",
    "REGEXP_INSTR",
    "REGEXP_COUNT",
    "REGEXP_REPLACE",
    "REGEXP_SPLIT_TO_ARRAY",
];

/// `RLIKE` is an operator keyword, not a function call — matched as a standalone keyword.
const RLIKE_KEYWORD: &str = "RLIKE";

impl Rule for RegexpFunction {
    fn name(&self) -> &'static str {
        "Ambiguous/RegexpFunction"
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

    // Scan for each regexp function name followed by '('
    for func_name in REGEXP_FUNCTIONS {
        scan_for_function(source, bytes, len, &skip, func_name, rule_name, &mut diags);
    }

    // Scan for RLIKE as a standalone keyword (operator form)
    scan_for_keyword(source, bytes, len, &skip, RLIKE_KEYWORD, rule_name, &mut diags);

    // Sort diagnostics by line then col for stable output
    diags.sort_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));

    diags
}

/// Scan for `func_name(` (case-insensitive) with word boundaries.
fn scan_for_function(
    source: &str,
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    func_name: &str,
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

        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            let after = i + kw_len;
            // Word boundary after: next char must not be a word char
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                // Check that after optional whitespace there is a '('
                let mut j = after;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < len && bytes[j] == b'(' {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: format!(
                            "{func} is dialect-specific regexp syntax — different databases use \
                             different regexp functions with inconsistent behavior; consider \
                             abstracting via a dbt macro",
                            func = func_name
                        ),
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

/// Scan for `RLIKE` as a standalone keyword (case-insensitive) with word boundaries.
fn scan_for_keyword(
    source: &str,
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    keyword: &str,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    let kw = keyword.as_bytes();
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
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                let (line, col) = line_col(source, i);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: format!(
                        "{func} is dialect-specific regexp syntax — different databases use \
                         different regexp functions with inconsistent behavior; consider \
                         abstracting via a dbt macro",
                        func = keyword
                    ),
                    line,
                    col,
                });
                i += kw_len;
                continue;
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
