use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct SpaceBeforeComma;

impl Rule for SpaceBeforeComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComma"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = ctx.source.as_bytes();
        let len = source.len();
        if len == 0 {
            return Vec::new();
        }

        let skip_map = SkipMap::build(&ctx.source);
        let mut diags = Vec::new();

        for i in 0..len {
            if source[i] != b',' {
                continue;
            }
            if !skip_map.is_code(i) {
                continue;
            }

            // If no preceding character, nothing to check.
            if i == 0 {
                continue;
            }

            let prev = source[i - 1];
            if prev != b' ' && prev != b'\t' {
                // No space before comma — not a violation.
                continue;
            }

            // There is at least one space/tab before the comma.
            // Determine whether this is leading-comma style:
            // scan backwards from (i-1) to find the nearest newline.
            // If every byte between that newline and i is whitespace, it's
            // a leading-comma — skip it.
            let mut is_leading_comma = true;
            let mut j = i.wrapping_sub(1);
            loop {
                let ch = source[j];
                if ch == b'\n' {
                    // Reached a newline — all chars between newline and comma
                    // were whitespace, so this is leading-comma style.
                    break;
                }
                if ch != b' ' && ch != b'\t' {
                    // Found non-whitespace before the comma on the same line.
                    is_leading_comma = false;
                    break;
                }
                if j == 0 {
                    // Reached start of file — everything before comma is whitespace.
                    break;
                }
                j -= 1;
            }

            if is_leading_comma {
                continue;
            }

            // Find the byte offset of the first space/tab before the comma.
            // Walk backwards from i-1 to find the run of spaces/tabs.
            let mut space_start = i - 1;
            while space_start > 0
                && (source[space_start - 1] == b' ' || source[space_start - 1] == b'\t')
            {
                space_start -= 1;
            }

            let (line, col) = byte_offset_to_line_col(&ctx.source, space_start);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "Remove space before comma".to_string(),
                line,
                col,
            });
        }

        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = ctx.source.as_bytes();
        let len = source.len();
        if len == 0 {
            return None;
        }

        let skip_map = SkipMap::build(&ctx.source);
        let mut result = Vec::with_capacity(len);
        let mut changed = false;

        // Collect the offsets of spaces that should be removed.
        // We mark bytes to skip in a separate pass.
        let mut remove = vec![false; len];

        for i in 0..len {
            if source[i] != b',' {
                continue;
            }
            if !skip_map.is_code(i) {
                continue;
            }
            if i == 0 {
                continue;
            }
            let prev = source[i - 1];
            if prev != b' ' && prev != b'\t' {
                continue;
            }

            // Check for leading-comma style.
            let mut is_leading_comma = true;
            let mut j = i.wrapping_sub(1);
            loop {
                let ch = source[j];
                if ch == b'\n' {
                    break;
                }
                if ch != b' ' && ch != b'\t' {
                    is_leading_comma = false;
                    break;
                }
                if j == 0 {
                    break;
                }
                j -= 1;
            }

            if is_leading_comma {
                continue;
            }

            // Mark the run of spaces/tabs before the comma for removal.
            let mut space_start = i - 1;
            while space_start > 0
                && (source[space_start - 1] == b' ' || source[space_start - 1] == b'\t')
            {
                space_start -= 1;
            }
            for k in space_start..i {
                remove[k] = true;
                changed = true;
            }
        }

        if !changed {
            return None;
        }

        for (idx, &byte) in source.iter().enumerate() {
            if !remove[idx] {
                result.push(byte);
            }
        }

        Some(String::from_utf8(result).expect("source was valid UTF-8"))
    }
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
