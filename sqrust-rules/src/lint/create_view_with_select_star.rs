use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct CreateViewWithSelectStar;

impl Rule for CreateViewWithSelectStar {
    fn name(&self) -> &'static str {
        "Lint/CreateViewWithSelectStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let lower = source.to_lowercase();
        let bytes = lower.as_bytes();
        let len = bytes.len();

        let mut diags = Vec::new();

        // Look for "create" keyword occurrences
        let create_kw = "create";
        let create_len = create_kw.len();
        let mut i = 0;

        while i + create_len <= len {
            if !skip.contains(&i) && lower[i..].starts_with(create_kw) {
                let before_ok = i == 0 || {
                    let b = bytes[i - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                let after_pos = i + create_len;
                let after_ok = after_pos >= len || {
                    let b = bytes[after_pos];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if before_ok && after_ok {
                    // Find the end of this statement
                    let stmt_end = find_stmt_end(&lower, i);
                    let window = &lower[i..stmt_end];

                    // The window must contain "view" (preceded by CREATE or CREATE OR REPLACE)
                    if window_contains_keyword(window, "view") {
                        // Check there is no table-like keyword between CREATE and VIEW
                        // that would indicate a CREATE TABLE ... AS ... pattern.
                        // We verify the pattern is strictly CREATE ... VIEW (no TABLE between)
                        let view_pos_in_window = find_keyword_in(window, "view");
                        let table_pos_in_window = find_keyword_in(window, "table");

                        let is_create_view = match (view_pos_in_window, table_pos_in_window) {
                            (Some(_), None) => true,
                            (Some(vp), Some(tp)) => vp < tp,
                            _ => false,
                        };

                        if is_create_view {
                            // Now check for "select *" or "select\t*" in the window
                            // after the VIEW keyword
                            if let Some(vp) = view_pos_in_window {
                                let after_view = &window[vp..];
                                if contains_select_star(after_view) {
                                    let (line, col) = offset_to_line_col(source, i);
                                    diags.push(Diagnostic {
                                        rule: self.name(),
                                        message: "CREATE VIEW with SELECT * is fragile; new \
                                                  columns added to underlying tables will not \
                                                  appear in the view"
                                            .to_string(),
                                        line,
                                        col,
                                    });
                                }
                            }
                        }
                    }
                }
                i += create_len;
            } else {
                i += 1;
            }
        }

        diags
    }
}

/// Returns true if the lowercased window contains "select *" or "select" followed
/// by optional whitespace/tabs and then "*" (but NOT "select *," patterns with extra
/// columns — actually per spec any SELECT * in the view body is flagged).
fn contains_select_star(window: &str) -> bool {
    let bytes = window.as_bytes();
    let len = bytes.len();
    let pat = "select";
    let pat_len = pat.len();
    let mut i = 0;
    while i + pat_len <= len {
        if window[i..].starts_with(pat) {
            // Word boundary before
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok {
                // Skip whitespace/tabs after "select"
                let mut j = i + pat_len;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                    j += 1;
                }
                if j < len && bytes[j] == b'*' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Finds the byte position of `keyword` (already lowercase) within `window`,
/// respecting word boundaries. Returns None if not found.
fn find_keyword_in(window: &str, keyword: &str) -> Option<usize> {
    let kw_len = keyword.len();
    let bytes = window.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i + kw_len <= len {
        if window[i..].starts_with(keyword) {
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after_pos = i + kw_len;
            let after_ok = after_pos >= len || {
                let b = bytes[after_pos];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Returns true if the lowercased window contains `keyword` with word boundaries.
fn window_contains_keyword(window: &str, keyword: &str) -> bool {
    find_keyword_in(window, keyword).is_some()
}

/// Returns the byte index just past the end of the current statement (i.e. the
/// position of the next `;` or the end of the source).
fn find_stmt_end(lower: &str, from: usize) -> usize {
    lower[from..]
        .find(';')
        .map(|rel| from + rel)
        .unwrap_or(lower.len())
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            // Single-quoted string — mark every byte inside as skip
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        // Escaped quote inside string
                        skip.insert(i);
                        i += 2;
                    } else {
                        // End of string
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            // Line comment — mark until end of line
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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
