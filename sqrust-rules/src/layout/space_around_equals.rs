use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct SpaceAroundEquals;

impl Rule for SpaceAroundEquals {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundEquals"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        if len == 0 {
            return None;
        }

        let skip_map = SkipMap::build(source);
        let mut result: Vec<u8> = Vec::with_capacity(len + 16);
        let mut changed = false;
        let mut i = 0;

        while i < len {
            let byte = bytes[i];

            if byte != b'=' || !skip_map.is_code(i) {
                result.push(byte);
                i += 1;
                continue;
            }

            // Compound operator checks
            let prev_byte = if i > 0 { bytes[i - 1] } else { b'\0' };
            let next_byte = if i + 1 < len { bytes[i + 1] } else { b'\0' };

            // Skip compound: !=, <=, >=
            if prev_byte == b'!' || prev_byte == b'<' || prev_byte == b'>' {
                result.push(byte);
                i += 1;
                continue;
            }
            // Skip => (fat arrow)
            if next_byte == b'>' {
                result.push(byte);
                i += 1;
                continue;
            }
            // Skip == (double equals)
            if next_byte == b'=' {
                result.push(byte);
                i += 1;
                continue;
            }
            // Skip if this is the second `=` of `==`
            if prev_byte == b'=' {
                result.push(byte);
                i += 1;
                continue;
            }

            // Determine what needs inserting
            let need_before = !is_whitespace(prev_byte) && prev_byte != b'\0';
            let need_after = !is_whitespace(next_byte) && next_byte != b'\0';

            if need_before {
                result.push(b' ');
                changed = true;
            }
            result.push(byte);
            if need_after {
                result.push(b' ');
                changed = true;
            }
            i += 1;
        }

        if !changed {
            return None;
        }

        Some(String::from_utf8(result).expect("source was valid UTF-8"))
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    if len == 0 {
        return Vec::new();
    }

    let skip_map = SkipMap::build(source);
    let mut diags = Vec::new();

    for i in 0..len {
        if bytes[i] != b'=' {
            continue;
        }
        if !skip_map.is_code(i) {
            continue;
        }

        let prev_byte = if i > 0 { bytes[i - 1] } else { b'\0' };
        let next_byte = if i + 1 < len { bytes[i + 1] } else { b'\0' };

        // Skip compound operators: !=, <=, >=
        if prev_byte == b'!' || prev_byte == b'<' || prev_byte == b'>' {
            continue;
        }
        // Skip => (fat arrow)
        if next_byte == b'>' {
            continue;
        }
        // Skip == and the second = of ==
        if next_byte == b'=' || prev_byte == b'=' {
            continue;
        }

        let missing_before = !is_whitespace(prev_byte) && prev_byte != b'\0';
        let missing_after = !is_whitespace(next_byte) && next_byte != b'\0';

        if missing_before || missing_after {
            let (line, col) = byte_offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "Operator '=' should have spaces on both sides".to_string(),
                line,
                col,
            });
        }
    }

    diags
}

#[inline]
fn is_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

/// Converts a byte offset into a 1-indexed (line, col) pair.
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = offset - line_start + 1;
    (line, col)
}
