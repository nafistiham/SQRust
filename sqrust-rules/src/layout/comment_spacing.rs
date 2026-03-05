use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CommentSpacing;

impl Rule for CommentSpacing {
    fn name(&self) -> &'static str {
        "Layout/CommentSpacing"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let violations = find_violations(&ctx.source, self.name());
        if violations.is_empty() {
            return None;
        }

        // Build a set of byte offsets where we need to insert a space.
        // Each violation points to the first `-` of `--`. We need to insert
        // a space at offset+2 (right after `--`).
        let bytes = ctx.source.as_bytes();
        let len = bytes.len();

        // Collect insert positions (byte offset of the character right after `--`).
        let mut inserts: Vec<usize> = Vec::new();
        // Re-scan to get byte offsets directly rather than going via line/col.
        let mut i = 0;
        let mut in_string = false;
        let mut block_depth: usize = 0;

        while i < len {
            // Single-quoted string
            if !in_string && block_depth == 0 && bytes[i] == b'\'' {
                in_string = true;
                i += 1;
                continue;
            }
            if in_string {
                if bytes[i] == b'\'' {
                    // SQL '' escape
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                i += 1;
                continue;
            }

            // Block comment open
            if block_depth == 0 && i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                block_depth += 1;
                i += 2;
                continue;
            }
            // Block comment close
            if block_depth > 0 {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    block_depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }

            // Line comment: `--`
            if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                let after = i + 2;
                // Check the byte immediately after `--`
                let next_byte = if after < len { Some(bytes[after]) } else { None };
                match next_byte {
                    // `---` or more dashes → divider, exempt
                    Some(b'-') => {}
                    // Space, newline, or EOF → OK (empty comment or has space)
                    None | Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'\t') => {}
                    // Any other character → violation; record insert position
                    Some(_) => {
                        inserts.push(after);
                    }
                }
                // Skip to end of line
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            i += 1;
        }

        if inserts.is_empty() {
            return None;
        }

        // Build fixed string by inserting a space at each recorded position.
        // inserts is in ascending order because we scanned left-to-right.
        let mut result = Vec::with_capacity(len + inserts.len());
        let mut prev = 0;
        for &pos in &inserts {
            result.extend_from_slice(&bytes[prev..pos]);
            result.push(b' ');
            prev = pos;
        }
        result.extend_from_slice(&bytes[prev..]);

        Some(String::from_utf8(result).expect("source was valid UTF-8"))
    }
}

/// Scan `source` and return Diagnostics for every `--` that is:
/// - outside single-quoted strings and block comments
/// - immediately followed by a non-space, non-newline, non-dash character
fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut i = 0;
    let mut in_string = false;
    let mut block_depth: usize = 0;

    while i < len {
        // Single-quoted string
        if !in_string && block_depth == 0 && bytes[i] == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\'' {
                // SQL '' escape
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Block comment open
        if block_depth == 0 && i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_depth += 1;
            i += 2;
            continue;
        }
        // Block comment content / close
        if block_depth > 0 {
            if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // Line comment: `--`
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let after = i + 2;
            let next_byte = if after < len { Some(bytes[after]) } else { None };
            match next_byte {
                // `---` or more dashes → divider, exempt
                Some(b'-') => {}
                // Space, tab, newline, CR, or EOF → OK
                None | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {}
                // Any other character → violation
                Some(_) => {
                    let (line, col) = byte_offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message:
                            "Line comment should have a space after '--'; write '-- comment'"
                                .to_string(),
                        line,
                        col,
                    });
                }
            }
            // Skip to end of line
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    diags
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
