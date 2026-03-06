use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct BlankLineBetweenStatements;

impl Rule for BlankLineBetweenStatements {
    fn name(&self) -> &'static str {
        "Layout/BlankLineBetweenStatements"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        // Find positions of all semicolons that are not inside string literals.
        let mut semi_positions: Vec<usize> = Vec::new();
        let mut in_string = false;
        let mut i = 0;

        while i < len {
            if !in_string && bytes[i] == b'\'' {
                in_string = true;
                i += 1;
                continue;
            }
            if in_string {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                i += 1;
                continue;
            }
            if bytes[i] == b';' {
                semi_positions.push(i);
            }
            i += 1;
        }

        // For each semicolon (except the last), check if the next non-whitespace
        // content after it is preceded by a blank line.
        for &semi_pos in &semi_positions {
            // Find the end of the current line (newline after semicolon)
            let mut j = semi_pos + 1;
            // Skip rest of current line
            while j < len && bytes[j] != b'\n' {
                j += 1;
            }
            if j >= len {
                // Semicolon at end of file — no next statement
                continue;
            }
            // j points to '\n' — move past it
            j += 1;
            if j >= len {
                continue;
            }

            // Count newlines in what follows — need at least one blank line
            // A blank line means two consecutive newlines
            let start_of_next_region = j;
            let mut blank_line_found = false;

            // Check if there's a blank line before the next content.
            // After advancing past the `;` line's own newline, the region starts
            // at what follows. A blank line means the very first line of the region
            // is empty (contains only spaces/tabs before its newline).
            let region = &source[start_of_next_region..];
            for c in region.chars() {
                match c {
                    '\n' => {
                        // First line of region ended without non-whitespace → blank line!
                        blank_line_found = true;
                        break;
                    }
                    ' ' | '\t' | '\r' => {
                        // whitespace on an otherwise blank line — keep scanning
                    }
                    _ => {
                        // non-whitespace before newline → this line is not blank
                        break;
                    }
                }
            }

            if !blank_line_found {
                // Find the start of the next statement (after the semicolon line)
                let next_stmt_offset = start_of_next_region
                    + region
                        .find(|c: char| !c.is_whitespace())
                        .unwrap_or(0);
                // Only flag if there actually is a next statement
                if region.chars().any(|c| !c.is_whitespace()) {
                    let (line, col) = offset_to_line_col(source, next_stmt_offset);
                    diags.push(Diagnostic {
                        rule: "Layout/BlankLineBetweenStatements",
                        message: "Statements must be separated by a blank line".to_string(),
                        line,
                        col,
                    });
                }
            }
        }

        diags
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
