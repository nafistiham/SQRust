use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ConsistentCommentStyle;

impl Rule for ConsistentCommentStyle {
    fn name(&self) -> &'static str {
        "Layout/ConsistentCommentStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

/// A comment occurrence: style and byte offset of its start.
#[derive(Clone, Copy, PartialEq)]
enum CommentStyle {
    Line,  // --
    Block, // /* ... */
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    let mut in_string = false;
    let mut i = 0usize;

    // Collect all comment occurrences as (style, byte_offset).
    let mut occurrences: Vec<(CommentStyle, usize)> = Vec::new();

    while i < len {
        let byte = bytes[i];

        // ── String tracking ────────────────────────────────────────────────
        if in_string {
            if byte == b'\'' {
                // SQL '' escape: two consecutive single-quotes inside a string
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Enter single-quoted string
        if byte == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }

        // ── Line comment: -- ───────────────────────────────────────────────
        if i + 1 < len && byte == b'-' && bytes[i + 1] == b'-' {
            occurrences.push((CommentStyle::Line, i));
            // Advance past the entire line
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // ── Block comment: /* ... */ ───────────────────────────────────────
        if i + 1 < len && byte == b'/' && bytes[i + 1] == b'*' {
            occurrences.push((CommentStyle::Block, i));
            i += 2; // move past /*
            // Advance past block comment body
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2; // move past */
                    break;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    if occurrences.is_empty() {
        return Vec::new();
    }

    // Count each style
    let line_count = occurrences.iter().filter(|(s, _)| *s == CommentStyle::Line).count();
    let block_count = occurrences.iter().filter(|(s, _)| *s == CommentStyle::Block).count();

    // If only one style is used, no violation
    if line_count == 0 || block_count == 0 {
        return Vec::new();
    }

    // Both styles present: determine which is the minority
    // When counts are equal, the "minority" is whichever style was seen SECOND
    // (i.e., the style that the first occurrence of the second style represents)
    let minority_style = if line_count < block_count {
        // line comments are rarer — flag the first line comment
        CommentStyle::Line
    } else if block_count < line_count {
        // block comments are rarer — flag the first block comment
        CommentStyle::Block
    } else {
        // Equal counts: flag the first occurrence of the style that appears second
        // The second style seen is the one that was NOT the first comment in the file
        let first_style = occurrences[0].0;
        if first_style == CommentStyle::Line {
            CommentStyle::Block
        } else {
            CommentStyle::Line
        }
    };

    // Find the first occurrence of the minority style
    let &(_, offset) = occurrences
        .iter()
        .find(|(s, _)| *s == minority_style)
        .expect("minority style has at least one occurrence");

    let (line, col) = byte_offset_to_line_col(source, offset);
    vec![Diagnostic {
        rule: rule_name,
        message: "Inconsistent comment style: file mixes -- and /* */ comments".to_string(),
        line,
        col,
    }]
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
