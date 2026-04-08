use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct AlterViewStatement;

impl Rule for AlterViewStatement {
    fn name(&self) -> &'static str {
        "Lint/AlterViewStatement"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let lower = source.to_lowercase();
        let bytes = lower.as_bytes();
        let len = bytes.len();
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if skip.contains(&i) {
                i += 1;
                continue;
            }

            if !lower[i..].starts_with("alter") {
                i += 1;
                continue;
            }

            let alter_start = i;
            let alter_end = i + 5; // "alter".len()

            // Word-boundary check before "alter".
            let before_ok = alter_start == 0 || {
                let b = bytes[alter_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after "alter".
            let after_ok = alter_end >= len || {
                let b = bytes[alter_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace between ALTER and next keyword.
            let mut j = alter_end;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\r'
                    || bytes[j] == b'\n')
                && !skip.contains(&j)
            {
                j += 1;
            }

            // Check that the very next keyword (with word boundary) is "view".
            let view_len = 4; // "view".len()
            if j + view_len > len {
                i += 1;
                continue;
            }

            let next_word_end = j + view_len;
            let next_word_ok = lower[j..].starts_with("view");

            let view_after_ok = next_word_end >= len || {
                let b = bytes[next_word_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if next_word_ok && view_after_ok {
                let stmt_end = find_stmt_end(&lower, &skip, next_word_end);
                let (line, col) = offset_to_line_col(source, alter_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "ALTER VIEW statements should not appear in SQL files".to_string(),
                    line,
                    col,
                });

                i = stmt_end + 1;
                continue;
            }

            i += 1;
        }

        diags
    }
}

fn find_stmt_end(lower: &str, skip: &HashSet<usize>, from: usize) -> usize {
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let mut k = from;
    while k < len {
        if !skip.contains(&k) && bytes[k] == b';' {
            return k;
        }
        k += 1;
    }
    len
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip.insert(i);
            skip.insert(i + 1);
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip.insert(i);
            skip.insert(i + 1);
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip.insert(i);
                    skip.insert(i + 1);
                    i += 2;
                    break;
                }
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote
        if bytes[i] == b'\'' {
            skip.insert(i);
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip.insert(i);
                    i += 1;
                    if i < len && bytes[i] == b'\'' {
                        skip.insert(i);
                        i += 1;
                        continue;
                    }
                    break;
                }
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            skip.insert(i);
            i += 1;
            while i < len && bytes[i] != b'"' {
                skip.insert(i);
                i += 1;
            }
            if i < len {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            skip.insert(i);
            i += 1;
            while i < len && bytes[i] != b'`' {
                skip.insert(i);
                i += 1;
            }
            if i < len {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
