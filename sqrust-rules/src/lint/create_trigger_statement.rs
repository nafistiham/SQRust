use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct CreateTriggerStatement;

impl Rule for CreateTriggerStatement {
    fn name(&self) -> &'static str {
        "Lint/CreateTriggerStatement"
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

            if !lower[i..].starts_with("create") {
                i += 1;
                continue;
            }

            let create_start = i;
            let create_end = i + 6; // "create".len()

            // Word-boundary check before "create".
            let before_ok = create_start == 0 || {
                let b = bytes[create_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after "create".
            let after_ok = create_end >= len || {
                let b = bytes[create_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace between CREATE and next keyword.
            let mut j = create_end;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\r'
                    || bytes[j] == b'\n')
                && !skip.contains(&j)
            {
                j += 1;
            }

            // Scan the rest of the statement for "trigger" keyword.
            let stmt_end = find_stmt_end(&lower, &skip, j);
            let stmt_slice = &lower[j..stmt_end];

            if contains_word_boundary_keyword(stmt_slice, "trigger") {
                let (line, col) = offset_to_line_col(source, create_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "CREATE TRIGGER statements should not appear in SQL files"
                        .to_string(),
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

fn contains_word_boundary_keyword(text: &str, keyword: &str) -> bool {
    let kw_len = keyword.len();
    let bytes = text.as_bytes();
    let text_len = bytes.len();
    let mut search_from = 0;

    while search_from < text_len {
        let Some(rel) = text[search_from..].find(keyword) else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + kw_len;
        let after_ok = after >= text_len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            return true;
        }
        search_from = abs + 1;
    }

    false
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
