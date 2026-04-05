use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct UndelimitedDateString;

impl Rule for UndelimitedDateString {
    fn name(&self) -> &'static str {
        "Ambiguous/UndelimitedDateString"
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
            // A string start is detected when: bytes[i] == '\'' AND
            //   (i == 0 OR skip.is_code(i - 1))
            if bytes[i] == b'\'' && (i == 0 || skip.is_code(i - 1)) {
                let str_start = i;
                i += 1;
                let content_start = i;
                // Walk forward while inside the string (skip == true).
                while i < len && !skip.is_code(i) {
                    i += 1;
                }
                // The closing quote was at i - 1.
                let content_end = if i > content_start { i - 1 } else { content_start };
                let content = &bytes[content_start..content_end];

                if let Some((year, month, day)) = detect_compact_date(content) {
                    let val = std::str::from_utf8(content).unwrap_or("");
                    let (line, col) = offset_to_line_col(source, str_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Date string '{val}' uses compact YYYYMMDD format without separators; \
                             prefer ISO 8601 with dashes: '{year}-{month}-{day}'"
                        ),
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

/// Returns `Some((year, month, day))` as string slices if the content is
/// exactly 8 ASCII digits forming a plausible YYYYMMDD date.
/// Year: 1000–2999, Month: 01–12, Day: 01–31.
fn detect_compact_date(content: &[u8]) -> Option<(&str, &str, &str)> {
    if content.len() != 8 {
        return None;
    }
    if !content.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let year_bytes = &content[0..4];
    let month_bytes = &content[4..6];
    let day_bytes = &content[6..8];

    let year: u32 = parse_u32(year_bytes)?;
    let month: u32 = parse_u32(month_bytes)?;
    let day: u32 = parse_u32(day_bytes)?;

    if year < 1000 || year > 2999 {
        return None;
    }
    if month < 1 || month > 12 {
        return None;
    }
    if day < 1 || day > 31 {
        return None;
    }

    Some((
        std::str::from_utf8(year_bytes).ok()?,
        std::str::from_utf8(month_bytes).ok()?,
        std::str::from_utf8(day_bytes).ok()?,
    ))
}

fn parse_u32(bytes: &[u8]) -> Option<u32> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
