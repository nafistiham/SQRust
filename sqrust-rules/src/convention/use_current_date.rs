use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct UseCurrentDate;

impl Rule for UseCurrentDate {
    fn name(&self) -> &'static str {
        "Convention/UseCurrentDate"
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

/// Whether a function requires `(` immediately after to be flagged, or can appear without parens.
enum MatchMode {
    /// Must be followed by `(` — e.g. GETDATE(), NOW()
    RequiresParen,
    /// May appear without `(` — e.g. SYSDATE in Oracle
    WordBoundaryOnly,
}

struct FunctionPattern {
    keyword: &'static [u8],
    message: &'static str,
    mode: MatchMode,
}

const PATTERNS: &[FunctionPattern] = &[
    FunctionPattern {
        keyword: b"SYSDATETIMEOFFSET",
        message: "SYSDATETIMEOFFSET() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL",
        mode: MatchMode::RequiresParen,
    },
    FunctionPattern {
        keyword: b"SYSDATETIME",
        message: "SYSDATETIME() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL",
        mode: MatchMode::RequiresParen,
    },
    FunctionPattern {
        keyword: b"GETUTCDATE",
        message: "GETUTCDATE() is SQL Server-specific; use CURRENT_TIMESTAMP AT TIME ZONE 'UTC' for standard SQL",
        mode: MatchMode::RequiresParen,
    },
    FunctionPattern {
        keyword: b"GETDATE",
        message: "GETDATE() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL",
        mode: MatchMode::RequiresParen,
    },
    FunctionPattern {
        keyword: b"SYSDATE",
        message: "SYSDATE is Oracle-specific; use CURRENT_DATE or CURRENT_TIMESTAMP for standard SQL",
        mode: MatchMode::WordBoundaryOnly,
    },
    FunctionPattern {
        keyword: b"NOW",
        message: "NOW() is MySQL/PostgreSQL-specific; use CURRENT_TIMESTAMP for standard SQL",
        mode: MatchMode::RequiresParen,
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

        // Try each pattern (ordered longest-first to avoid prefix collisions)
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

            match pattern.mode {
                MatchMode::RequiresParen => {
                    // Must be immediately followed by '('
                    if kw_end >= len || bytes[kw_end] != b'(' {
                        continue;
                    }
                }
                MatchMode::WordBoundaryOnly => {
                    // Word boundary after: next char must not be a word char
                    let after_ok = kw_end >= len || !is_word_char(bytes[kw_end]);
                    if !after_ok {
                        continue;
                    }
                }
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
