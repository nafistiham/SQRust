use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct WhitespaceBeforeSemicolon;

impl Rule for WhitespaceBeforeSemicolon {
    fn name(&self) -> &'static str {
        "Layout/WhitespaceBeforeSemicolon"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        Some(apply_fix(&ctx.source))
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut in_string = false;
    let mut i = 0usize;

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

        // ── Skip -- line comment to end of line ────────────────────────────
        if i + 1 < len && byte == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // ── Skip /* ... */ block comment ───────────────────────────────────
        if i + 1 < len && byte == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // ── Semicolon check ────────────────────────────────────────────────
        if byte == b';' && i > 0 {
            let prev = bytes[i - 1];
            if prev == b' ' || prev == b'\t' {
                // Find the start of the whitespace run before the semicolon
                let mut ws_start = i - 1;
                while ws_start > 0 && (bytes[ws_start - 1] == b' ' || bytes[ws_start - 1] == b'\t') {
                    ws_start -= 1;
                }
                let (line, col) = byte_offset_to_line_col(source, ws_start);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "Unexpected whitespace before semicolon".to_string(),
                    line,
                    col,
                });
            }
        }

        i += 1;
    }

    diags
}

/// Remove all whitespace (spaces and tabs) immediately before each semicolon
/// (outside string literals and comments).
fn apply_fix(source: &str) -> String {
    let bytes = source.as_bytes();
    let len = bytes.len();

    // Build a set of byte indices that are whitespace-before-semicolon to remove.
    // We track characters to remove, then reconstruct.
    let mut remove: Vec<bool> = vec![false; len];

    let mut in_string = false;
    let mut i = 0usize;

    while i < len {
        let byte = bytes[i];

        if in_string {
            if byte == b'\'' {
                if i + 1 < len && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        if byte == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }

        // Skip line comments
        if i + 1 < len && byte == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip block comments
        if i + 1 < len && byte == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        if byte == b';' && i > 0 {
            // Mark all preceding whitespace for removal
            let mut j = i - 1;
            loop {
                if bytes[j] == b' ' || bytes[j] == b'\t' {
                    remove[j] = true;
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                } else {
                    break;
                }
            }
        }

        i += 1;
    }

    // Reconstruct source without removed bytes
    bytes
        .iter()
        .enumerate()
        .filter(|(idx, _)| !remove[*idx])
        .map(|(_, &b)| b as char)
        .collect()
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
