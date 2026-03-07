use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NullsOrdering;

impl Rule for NullsOrdering {
    fn name(&self) -> &'static str {
        "Ambiguous/NullsOrdering"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let positions = find_order_by_positions(source);
        let mut diags = Vec::new();

        for order_by_offset in positions {
            // Scan forward from the ORDER BY position to the next `;` or end of source.
            let region_end = source[order_by_offset..]
                .find(';')
                .map(|rel| order_by_offset + rel)
                .unwrap_or(source.len());
            let region = &source[order_by_offset..region_end];

            // Check whether `NULLS` appears in this region (word boundary, case-insensitive).
            if !contains_nulls_keyword(region) {
                let (line, col) = offset_to_line_col(source, order_by_offset);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "ORDER BY without NULLS FIRST/NULLS LAST is ambiguous; NULL sort order varies by database".to_string(),
                    line,
                    col,
                });
            }
        }

        diags
    }
}

/// Finds all byte offsets where `ORDER BY` appears outside string literals,
/// with a word boundary before the `O`.
fn find_order_by_positions(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let upper = source.to_ascii_uppercase();
    let upper_bytes = upper.as_bytes();
    let mut positions = Vec::new();
    let mut in_string = false;
    let mut i = 0;

    while i < bytes.len() {
        // Handle single-quoted string literals (SQL strings).
        // Escaped quote inside a string: two consecutive single quotes `''`.
        if !in_string && bytes[i] == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\'' {
                // Peek ahead for escaped quote `''`.
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Try to match `ORDER BY` at a word boundary.
        // "ORDER BY" is 8 characters.
        if i + 8 <= upper_bytes.len() && &upper_bytes[i..i + 8] == b"ORDER BY" {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            if before_ok {
                positions.push(i);
                i += 8;
                continue;
            }
        }

        i += 1;
    }

    positions
}

/// Returns true if `region` contains the word `NULLS` (case-insensitive, word boundary).
fn contains_nulls_keyword(region: &str) -> bool {
    let bytes = region.as_bytes();
    let upper = region.to_ascii_uppercase();
    let upper_bytes = upper.as_bytes();
    let kw = b"NULLS";
    let kw_len = kw.len();

    let mut i = 0;
    while i + kw_len <= upper_bytes.len() {
        if &upper_bytes[i..i + kw_len] == kw {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
