use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct AmbiguousDateFormat;

impl Rule for AmbiguousDateFormat {
    fn name(&self) -> &'static str {
        "Ambiguous/AmbiguousDateFormat"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Find a single-quote that begins a string literal.
            // The SkipMap marks the opening quote as skip (not code), but the
            // byte before it (or the start of source) is code.
            // A string start is detected when: bytes[i] == '\'' AND
            //   (i == 0 OR skip.is_code(i - 1))
            if bytes[i] == b'\'' && (i == 0 || skip.is_code(i - 1)) {
                let str_start = i;
                // Step past the opening quote — the SkipMap already consumed the whole string,
                // so find the closing quote by advancing through non-code bytes.
                i += 1;
                let content_start = i;
                // Walk forward while the bytes are inside the string (skip == true)
                // or until we hit the matching closing quote.
                // The closing quote is also marked as skip, and the byte after it is code again.
                while i < len && !skip.is_code(i) {
                    i += 1;
                }
                // At this point i is either past the end or at a code byte.
                // The closing quote was at i - 1 (marked skip).
                let content_end = if i > content_start { i - 1 } else { content_start };
                let content = &bytes[content_start..content_end];

                if is_ambiguous_slash_date(content) {
                    let (line, col) = offset_to_line_col(source, str_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "Date literal uses slash-separated format which is locale-dependent (MM/DD vs DD/MM); use ISO 8601 format ('YYYY-MM-DD') instead".to_string(),
                        line,
                        col,
                    });
                }
                continue;
            }
            i += 1;
        }

        diags
    }
}

/// Returns true if `s` looks like an ambiguous slash-separated date.
/// Pattern: 1-2 digits / 1-2 digits / 2-4 digits, where the first segment <= 31 (not a year).
fn is_ambiguous_slash_date(s: &[u8]) -> bool {
    // Trim whitespace
    let s = trim_bytes(s);
    if s.len() < 5 { return false; }

    // Parse first segment
    let (seg1, rest) = read_digits(s);
    if seg1.is_empty() || seg1.len() > 2 { return false; }
    let n1: u32 = match std::str::from_utf8(seg1).ok().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return false,
    };
    if n1 > 31 { return false; } // year-first format — not ambiguous

    // Must have slash
    if rest.is_empty() || rest[0] != b'/' { return false; }
    let rest = &rest[1..];

    // Parse second segment
    let (seg2, rest) = read_digits(rest);
    if seg2.is_empty() || seg2.len() > 2 { return false; }

    // Must have slash
    if rest.is_empty() || rest[0] != b'/' { return false; }
    let rest = &rest[1..];

    // Parse third segment (year: exactly 2 or 4 digits)
    let (seg3, rest) = read_digits(rest);
    if !(seg3.len() == 2 || seg3.len() == 4) { return false; }

    // Must be end of string (allow time component after space)
    if !rest.is_empty() && rest[0] != b' ' && rest[0] != b'T' {
        return false;
    }

    true
}

fn read_digits(s: &[u8]) -> (&[u8], &[u8]) {
    let end = s.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

fn trim_bytes(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(0);
    let end = s.iter().rposition(|&b| b != b' ' && b != b'\t').map(|i| i + 1).unwrap_or(0);
    if start >= end { &[] } else { &s[start..end] }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
