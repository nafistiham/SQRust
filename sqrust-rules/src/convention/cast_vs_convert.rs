use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CastVsConvert;

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Return the byte offsets of every `CONVERT(` token (case-insensitive, whole-word boundary)
/// in `source`.
fn find_all_convert(source: &str) -> Vec<usize> {
    // We look for the 7-char sequence "CONVERT" followed by '('.
    let bytes = source.as_bytes();
    let name = b"CONVERT";
    let name_len = name.len();
    let len = bytes.len();
    let mut offsets = Vec::new();
    let mut i = 0;

    while i + name_len < len {
        // Word boundary before.
        let before_ok = i == 0
            || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok {
            let matches = bytes[i..i + name_len]
                .iter()
                .zip(name.iter())
                .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

            if matches {
                // Must be immediately followed by '(' to be a function call.
                let after = i + name_len;
                if after < len && bytes[after] == b'(' {
                    offsets.push(i);
                    // Skip past this match to avoid re-scanning the same position.
                    i += name_len;
                    continue;
                }
            }
        }

        i += 1;
    }

    offsets
}

impl Rule for CastVsConvert {
    fn name(&self) -> &'static str {
        "Convention/CastVsConvert"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Hybrid approach: use parse_errors to bail on unparseable files,
        // then scan source text for CONVERT( occurrences.
        // sqlparser-rs parses CONVERT() as Expr::Convert (a dedicated AST node), but
        // nested CONVERT arguments are embedded in data_type fields and not reachable
        // via normal expression walking. Source scanning is the reliable approach.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        find_all_convert(&ctx.source)
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(&ctx.source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "CONVERT() is dialect-specific — use CAST(expression AS type) for portable type conversion".to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
