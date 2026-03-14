use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct StringToNumberComparison;

impl Rule for StringToNumberComparison {
    fn name(&self) -> &'static str {
        "Ambiguous/StringToNumberComparison"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        find_violations(source, self.name())
    }
}

/// State of the scanner position relative to SQL constructs.
#[derive(PartialEq)]
enum ScanState {
    Code,
    InDoubleQuoteIdent,
    InLineComment,
    InBlockComment,
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let mut diags = Vec::new();
    let mut i = 0;
    let mut line: usize = 1;
    let mut line_start: usize = 0;
    let mut state = ScanState::Code;

    while i < len {
        match state {
            ScanState::InLineComment => {
                if bytes[i] == b'\n' {
                    line += 1;
                    line_start = i + 1;
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::InBlockComment => {
                if bytes[i] == b'\n' {
                    line += 1;
                    line_start = i + 1;
                }
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    state = ScanState::Code;
                } else {
                    i += 1;
                }
            }
            ScanState::InDoubleQuoteIdent => {
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 2; // escaped quote
                    } else {
                        i += 1;
                        state = ScanState::Code;
                    }
                } else {
                    i += 1;
                }
            }
            ScanState::Code => {
                // Track newlines for line/col
                if bytes[i] == b'\n' {
                    line += 1;
                    line_start = i + 1;
                    i += 1;
                    continue;
                }

                // Enter line comment
                if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                    state = ScanState::InLineComment;
                    i += 2;
                    continue;
                }

                // Enter block comment
                if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    state = ScanState::InBlockComment;
                    i += 2;
                    continue;
                }

                // Enter double-quoted identifier
                if bytes[i] == b'"' {
                    state = ScanState::InDoubleQuoteIdent;
                    i += 1;
                    continue;
                }

                // Pattern 1: 'string' OP integer/decimal
                if bytes[i] == b'\'' {
                    let token_start = i;
                    let token_line = line;
                    let token_col = token_start - line_start + 1;

                    // Scan past the single-quoted string
                    i += 1; // skip opening quote
                    while i < len {
                        if bytes[i] == b'\'' {
                            if i + 1 < len && bytes[i + 1] == b'\'' {
                                i += 2; // escaped
                                continue;
                            }
                            i += 1; // past closing quote
                            break;
                        }
                        i += 1;
                    }

                    // Skip whitespace
                    skip_ws_tracking(bytes, len, &mut i, &mut line, &mut line_start);

                    if i >= len {
                        continue;
                    }

                    // Try to match comparison operator
                    let (op_matched, after_op) = match_comparison_op(bytes, len, i);
                    if !op_matched {
                        // No comparison op — reset to just after token
                        // (i is already past the string token, continue from there)
                        continue;
                    }
                    i = after_op;

                    // Skip whitespace after operator
                    skip_ws_tracking(bytes, len, &mut i, &mut line, &mut line_start);

                    if i >= len {
                        continue;
                    }

                    // Check for numeric literal
                    if is_numeric_literal(bytes, len, i) {
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: "Comparing a string literal to a numeric literal may cause implicit type coercion with dialect-specific results; use explicit CAST".to_string(),
                            line: token_line,
                            col: token_col,
                        });
                    }
                    continue;
                }

                // Pattern 2: integer/decimal OP 'string'
                if bytes[i].is_ascii_digit() {
                    let num_start = i;
                    let num_line = line;
                    let num_col = num_start - line_start + 1;

                    let num_end = skip_numeric_literal(bytes, len, i);
                    if num_end == num_start {
                        i += 1;
                        continue;
                    }
                    i = num_end;

                    // Skip whitespace
                    skip_ws_tracking(bytes, len, &mut i, &mut line, &mut line_start);

                    if i >= len {
                        continue;
                    }

                    // Try to match comparison operator
                    let (op_matched, after_op) = match_comparison_op(bytes, len, i);
                    if !op_matched {
                        continue;
                    }
                    i = after_op;

                    // Skip whitespace after operator
                    skip_ws_tracking(bytes, len, &mut i, &mut line, &mut line_start);

                    if i >= len {
                        continue;
                    }

                    // Check for single-quoted string literal
                    if bytes[i] == b'\'' {
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: "Comparing a string literal to a numeric literal may cause implicit type coercion with dialect-specific results; use explicit CAST".to_string(),
                            line: num_line,
                            col: num_col,
                        });
                    }
                    continue;
                }

                i += 1;
            }
        }
    }

    diags
}

/// Skip ASCII whitespace, tracking line and line_start.
fn skip_ws_tracking(
    bytes: &[u8],
    len: usize,
    i: &mut usize,
    line: &mut usize,
    line_start: &mut usize,
) {
    while *i < len {
        match bytes[*i] {
            b' ' | b'\t' | b'\r' => {
                *i += 1;
            }
            b'\n' => {
                *line += 1;
                *line_start = *i + 1;
                *i += 1;
            }
            _ => break,
        }
    }
}

/// Returns (matched, position_after_op) for comparison operators: =, !=, <>, <=, >=, <, >
fn match_comparison_op(bytes: &[u8], len: usize, i: usize) -> (bool, usize) {
    if i >= len {
        return (false, i);
    }

    // Two-character operators first
    if i + 1 < len {
        let pair = (bytes[i], bytes[i + 1]);
        match pair {
            (b'!', b'=') | (b'<', b'>') | (b'<', b'=') | (b'>', b'=') => {
                return (true, i + 2);
            }
            _ => {}
        }
    }

    // Single-character operators
    match bytes[i] {
        b'=' | b'<' | b'>' => (true, i + 1),
        _ => (false, i),
    }
}

/// Returns true if there is a numeric literal (integer or decimal) at `i`.
fn is_numeric_literal(bytes: &[u8], len: usize, i: usize) -> bool {
    if i >= len || !bytes[i].is_ascii_digit() {
        return false;
    }
    let mut j = i;
    while j < len && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j < len && bytes[j] == b'.' {
        j += 1;
        while j < len && bytes[j].is_ascii_digit() {
            j += 1;
        }
    }
    // Must be followed by word boundary
    j >= len || !is_word_char(bytes[j])
}

/// Returns position just past the numeric literal at `i`, or `i` if not a number.
fn skip_numeric_literal(bytes: &[u8], len: usize, i: usize) -> usize {
    if i >= len || !bytes[i].is_ascii_digit() {
        return i;
    }
    let mut j = i;
    while j < len && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j < len && bytes[j] == b'.' {
        j += 1;
        while j < len && bytes[j].is_ascii_digit() {
            j += 1;
        }
    }
    // If followed by a word char, it's an identifier — not a numeric literal
    if j < len && is_word_char(bytes[j]) {
        return i;
    }
    j
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
