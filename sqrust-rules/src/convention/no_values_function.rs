use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoValuesFunction;

impl Rule for NoValuesFunction {
    fn name(&self) -> &'static str {
        "Convention/NoValuesFunction"
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

/// Check if the byte slice starting at `pos` (within `bytes`) contains `keyword`
/// (case-insensitive) as a complete word. Returns true if found.
fn contains_word_ci(bytes: &[u8], pos: usize, end: usize, keyword: &[u8]) -> bool {
    let kw_len = keyword.len();
    if end < kw_len {
        return false;
    }
    let mut i = pos;
    while i + kw_len <= end {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(keyword) {
            let after = i + kw_len;
            let after_ok = after >= end || !is_word_char(bytes[after]);
            if after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // VALUES keyword length
    let values_kw = b"VALUES";
    let values_len = values_kw.len();

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Try to match VALUES at position i with word boundary before
        if i + values_len > len {
            break;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        if !bytes[i..i + values_len].eq_ignore_ascii_case(values_kw) {
            i += 1;
            continue;
        }

        // Ensure all VALUES chars are code (not in string/comment)
        let all_code = (0..values_len).all(|k| !skip.contains(&(i + k)));
        if !all_code {
            i += 1;
            continue;
        }

        let values_end = i + values_len;

        // Word boundary after VALUES must be `(` for it to be a function call
        if values_end >= len || bytes[values_end] != b'(' {
            i += 1;
            continue;
        }

        // Found `VALUES(`. Now determine if this is a function call (not INSERT clause).
        // Look back up to 300 characters for expression-context keywords.
        let window_start = if i >= 300 { i - 300 } else { 0 };
        let context_slice = &bytes[window_start..i];

        // Expression context keywords that indicate VALUES() function usage:
        // ON DUPLICATE KEY UPDATE (most common case)
        // SET (UPDATE ... SET col = VALUES(col))
        // AND, OR, THEN, ELSE, WHERE (general expression context)
        let is_expression_context = contains_word_ci(context_slice, 0, context_slice.len(), b"UPDATE")
            || contains_word_ci(context_slice, 0, context_slice.len(), b"SET");

        if is_expression_context {
            // Additional check: make sure there's an INSERT in the context which means
            // this is INSERT...VALUES clause, not the VALUES() function.
            // If INSERT is present but VALUES( appears AFTER ON DUPLICATE KEY UPDATE,
            // then it IS the function.
            // We detect this by checking if ON DUPLICATE KEY UPDATE appears in context.
            let has_on_duplicate = contains_word_ci(context_slice, 0, context_slice.len(), b"DUPLICATE");
            let has_insert = contains_word_ci(context_slice, 0, context_slice.len(), b"INSERT");

            // If INSERT is present and DUPLICATE is not, this might be INSERT SET (MySQL extension)
            // or UPDATE SET. Flag it either way if UPDATE/SET is present.
            let _ = has_insert; // captured for clarity; we flag based on expression context
            let _ = has_on_duplicate;

            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "VALUES() function is MySQL-specific (used in ON DUPLICATE KEY UPDATE) — not supported in other databases".to_string(),
                line,
                col,
            });
        }

        i = values_end + 1;
    }

    diags
}
