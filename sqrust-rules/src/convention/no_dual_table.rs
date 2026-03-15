use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoDualTable;

const MESSAGE: &str =
    "FROM DUAL is Oracle-specific; omit the FROM clause or use FROM (VALUES (1)) AS dual for standard SQL";

impl Rule for NoDualTable {
    fn name(&self) -> &'static str {
        "Convention/NoDualTable"
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

    // "FROM" is 4 characters, "DUAL" is 4 characters
    let from_kw = b"FROM";
    let from_len = from_kw.len();
    let dual_kw = b"DUAL";
    let dual_len = dual_kw.len();

    let mut i = 0;
    while i + from_len <= len {
        // Skip positions inside string literals or comments
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Check word boundary before FROM
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match of "FROM"
        if !bytes[i..i + from_len].eq_ignore_ascii_case(from_kw) {
            i += 1;
            continue;
        }

        // Ensure none of the FROM keyword bytes are in string/comment
        let from_all_code = (0..from_len).all(|k| !skip.contains(&(i + k)));
        if !from_all_code {
            i += 1;
            continue;
        }

        // FROM must be followed by a word boundary (not inside a word like "FROMAGE")
        let from_end = i + from_len;
        if from_end < len && is_word_char(bytes[from_end]) {
            i += 1;
            continue;
        }

        // Skip any whitespace (including newlines) after FROM
        let mut j = from_end;
        while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
            j += 1;
        }

        // Check if DUAL follows
        if j + dual_len > len {
            i += 1;
            continue;
        }

        if !bytes[j..j + dual_len].eq_ignore_ascii_case(dual_kw) {
            i += 1;
            continue;
        }

        // DUAL must be a standalone word (word boundary after)
        let dual_end = j + dual_len;
        let after_dual_ok = dual_end >= len || !is_word_char(bytes[dual_end]);
        if !after_dual_ok {
            i += 1;
            continue;
        }

        // Ensure none of the DUAL bytes are in string/comment
        let dual_all_code = (0..dual_len).all(|k| !skip.contains(&(j + k)));
        if !dual_all_code {
            i += 1;
            continue;
        }

        // Report violation at the FROM keyword position
        let (line, col) = line_col(source, i);
        diags.push(Diagnostic {
            rule: rule_name,
            message: MESSAGE.to_string(),
            line,
            col,
        });

        i = dual_end;
    }

    diags
}
