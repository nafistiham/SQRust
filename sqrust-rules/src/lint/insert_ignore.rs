use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct InsertIgnore;

impl Rule for InsertIgnore {
    fn name(&self) -> &'static str {
        "Lint/InsertIgnore"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let mut diags = Vec::new();

        // Check for INSERT OR IGNORE first (longer pattern must come before INSERT IGNORE)
        for (line, col) in find_keyword(source, "insert or ignore", &skip) {
            diags.push(Diagnostic {
                rule: self.name(),
                message: "INSERT OR IGNORE is SQLite-specific — use INSERT with explicit \
                          duplicate handling or ON CONFLICT clauses"
                    .to_string(),
                line,
                col,
            });
        }

        // Check for INSERT IGNORE (but not positions already covered by INSERT OR IGNORE)
        let or_ignore_positions: HashSet<(usize, usize)> =
            find_keyword(source, "insert or ignore", &skip)
                .into_iter()
                .collect();

        for (line, col) in find_keyword(source, "insert ignore", &skip) {
            if !or_ignore_positions.contains(&(line, col)) {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "INSERT IGNORE is MySQL-specific — use INSERT with explicit \
                              duplicate handling or ON CONFLICT clauses"
                        .to_string(),
                    line,
                    col,
                });
            }
        }

        diags.sort_by_key(|d| (d.line, d.col));
        diags
    }
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
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

fn find_keyword(source: &str, keyword: &str, skip: &HashSet<usize>) -> Vec<(usize, usize)> {
    let lower = source.to_lowercase();
    let kw_len = keyword.len();
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;
    while i + kw_len <= len {
        if !skip.contains(&i) && lower[i..].starts_with(keyword) {
            let before_ok = i == 0
                || {
                    let b = bytes[i - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
            let after_pos = i + kw_len;
            let after_ok = after_pos >= len
                || {
                    let b = bytes[after_pos];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
            if before_ok && after_ok {
                let (line, col) = offset_to_line_col(source, i);
                results.push((line, col));
            }
        }
        i += 1;
    }
    results
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
