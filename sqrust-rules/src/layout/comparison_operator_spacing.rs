use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ComparisonOperatorSpacing;

impl Rule for ComparisonOperatorSpacing {
    fn name(&self) -> &'static str {
        "Layout/ComparisonOperatorSpacing"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut diags = Vec::new();

    let mut i = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut block_depth: usize = 0;

    while i < len {
        // Reset line comment at newline
        if bytes[i] == b'\n' {
            in_line_comment = false;
            i += 1;
            continue;
        }

        // Skip line comment content
        if in_line_comment {
            i += 1;
            continue;
        }

        // Single-quoted string handling (outside block comments)
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

        // Block comment open: /*
        if block_depth == 0 && i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_depth += 1;
            i += 2;
            continue;
        }
        // Inside block comment
        if block_depth > 0 {
            if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // Line comment start: --
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            in_line_comment = true;
            i += 2;
            continue;
        }

        // ── Operator detection ────────────────────────────────────────────────

        // Check for multi-character operators first to avoid double-flagging
        // their component characters.

        // !=  (two bytes: ! =)
        if bytes[i] == b'!' && i + 1 < len && bytes[i + 1] == b'=' {
            let op = "!=";
            if !well_spaced(bytes, i, 2) {
                let (line, col) = byte_offset_to_line_col(source, i);
                diags.push(make_diag(rule_name, op, line, col));
            }
            i += 2;
            continue;
        }

        // <>, <=, >=  — all start with < or >
        if bytes[i] == b'<' {
            // Skip <<  (bit-shift)
            if i + 1 < len && bytes[i + 1] == b'<' {
                i += 2;
                continue;
            }
            // Skip ->  (not applicable to '<', but skip =>)
            // Handle <>, <=
            if i + 1 < len && (bytes[i + 1] == b'>' || bytes[i + 1] == b'=') {
                let op = if bytes[i + 1] == b'>' { "<>" } else { "<=" };
                if !well_spaced(bytes, i, 2) {
                    let (line, col) = byte_offset_to_line_col(source, i);
                    diags.push(make_diag(rule_name, op, line, col));
                }
                i += 2;
                continue;
            }
            // Skip <!  (XML/HTML-like: <!)
            if i + 1 < len && bytes[i + 1] == b'!' {
                i += 1;
                continue;
            }
            // Skip </  (XML/HTML-like: </)
            if i + 1 < len && bytes[i + 1] == b'/' {
                i += 1;
                continue;
            }
            // Single <
            if !well_spaced(bytes, i, 1) {
                let (line, col) = byte_offset_to_line_col(source, i);
                diags.push(make_diag(rule_name, "<", line, col));
            }
            i += 1;
            continue;
        }

        if bytes[i] == b'>' {
            // Skip >>  (bit-shift)
            if i + 1 < len && bytes[i + 1] == b'>' {
                i += 2;
                continue;
            }
            // >=
            if i + 1 < len && bytes[i + 1] == b'=' {
                let op = ">=";
                if !well_spaced(bytes, i, 2) {
                    let (line, col) = byte_offset_to_line_col(source, i);
                    diags.push(make_diag(rule_name, op, line, col));
                }
                i += 2;
                continue;
            }
            // Single >  — skip => (fat arrow, handled by prior '=' check if any)
            // The '>' here could be part of '->' or '=>'. Check the byte before.
            // '->' : byte before '>' is '-'
            // '=>' : byte before '>' is '='
            let prev = if i > 0 { bytes[i - 1] } else { b' ' };
            if prev == b'-' || prev == b'=' {
                // part of -> or => — skip
                i += 1;
                continue;
            }
            if !well_spaced(bytes, i, 1) {
                let (line, col) = byte_offset_to_line_col(source, i);
                diags.push(make_diag(rule_name, ">", line, col));
            }
            i += 1;
            continue;
        }

        i += 1;
    }

    diags
}

/// Returns true if the operator of `op_len` bytes starting at `pos` is
/// surrounded by whitespace (or start/end of source) on both sides.
fn well_spaced(bytes: &[u8], pos: usize, op_len: usize) -> bool {
    let before_ok = if pos == 0 {
        true
    } else {
        let b = bytes[pos - 1];
        b == b' ' || b == b'\t'
    };

    let after_pos = pos + op_len;
    let after_ok = if after_pos >= bytes.len() {
        true
    } else {
        let b = bytes[after_pos];
        b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
    };

    before_ok && after_ok
}

fn make_diag(rule_name: &'static str, op: &str, line: usize, col: usize) -> Diagnostic {
    Diagnostic {
        rule: rule_name,
        message: format!("Missing spaces around comparison operator '{op}'"),
        line,
        col,
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
