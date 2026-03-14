use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoCharindexFunction;

impl Rule for NoCharindexFunction {
    fn name(&self) -> &'static str {
        "Convention/NoCharindexFunction"
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

struct FunctionPattern {
    keyword: &'static [u8],
    message: &'static str,
}

const PATTERNS: &[FunctionPattern] = &[
    FunctionPattern {
        keyword: b"CHARINDEX",
        message: "CHARINDEX() is SQL Server-specific; use POSITION(substring IN string) for standard SQL",
    },
    FunctionPattern {
        keyword: b"LOCATE",
        message: "LOCATE() is MySQL-specific; use POSITION(substring IN string) for standard SQL",
    },
    FunctionPattern {
        keyword: b"INSTR",
        message: "INSTR() is Oracle/MySQL-specific; use POSITION(substring IN string) for standard SQL",
    },
];

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Word boundary check before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Try each pattern
        let mut matched = false;
        for pattern in PATTERNS {
            let kw_len = pattern.keyword.len();
            if i + kw_len > len {
                continue;
            }

            if !bytes[i..i + kw_len].eq_ignore_ascii_case(pattern.keyword) {
                continue;
            }

            // Ensure all keyword bytes are outside string/comment
            let all_code = (0..kw_len).all(|k| !skip.contains(&(i + k)));
            if !all_code {
                continue;
            }

            let kw_end = i + kw_len;

            // Must be immediately followed by '(' to be a function call
            if kw_end >= len || bytes[kw_end] != b'(' {
                continue;
            }

            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: pattern.message.to_string(),
                line,
                col,
            });

            i = kw_end + 1;
            matched = true;
            break;
        }

        if !matched {
            i += 1;
        }
    }

    diags
}
