use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct UnicodeIdentifiers;

impl Rule for UnicodeIdentifiers {
    fn name(&self) -> &'static str {
        "Layout/UnicodeIdentifiers"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse entirely.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;

        // We walk char by char, tracking whether we are inside:
        //   - a single-quoted string  (skip)
        //   - a double-quoted identifier (skip)
        //   - a block comment /* ... */ (skip)
        //   - a line comment  -- ... \n  (skip)
        //
        // For every non-ASCII character that falls *outside* all skip contexts,
        // we emit one diagnostic.

        let chars: Vec<char> = source.chars().collect();
        let n = chars.len();

        let mut i = 0usize;
        // Track line/col (1-indexed).  We advance these as we scan.
        let mut line: usize = 1;
        let mut col: usize = 1;

        // Skip-context flags
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut in_block_comment = false;
        let mut in_line_comment = false;

        while i < n {
            let ch = chars[i];

            // --- Detect context transitions ---

            // Exit line comment on newline
            if in_line_comment {
                if ch == '\n' {
                    in_line_comment = false;
                }
                // advance position and continue
                if ch == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
                continue;
            }

            // Exit block comment on */
            if in_block_comment {
                if ch == '*' && i + 1 < n && chars[i + 1] == '/' {
                    // consume both chars
                    col += 1; i += 1; // '*'
                    col += 1; i += 1; // '/'
                    in_block_comment = false;
                } else {
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                    i += 1;
                }
                continue;
            }

            // Inside single-quoted string: exit on unescaped '
            if in_single_quote {
                if ch == '\'' {
                    // Standard SQL uses '' to escape a quote — peek ahead
                    if i + 1 < n && chars[i + 1] == '\'' {
                        // escaped quote: consume both
                        col += 1; i += 1;
                        col += 1; i += 1;
                    } else {
                        in_single_quote = false;
                        col += 1;
                        i += 1;
                    }
                } else {
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                    i += 1;
                }
                continue;
            }

            // Inside double-quoted identifier: exit on "
            if in_double_quote {
                if ch == '"' {
                    if i + 1 < n && chars[i + 1] == '"' {
                        // escaped double-quote inside identifier
                        col += 1; i += 1;
                        col += 1; i += 1;
                    } else {
                        in_double_quote = false;
                        col += 1;
                        i += 1;
                    }
                } else {
                    if ch == '\n' {
                        line += 1;
                        col = 1;
                    } else {
                        col += 1;
                    }
                    i += 1;
                }
                continue;
            }

            // Not inside any skip context — check for context-entry or flagging.

            // Enter block comment
            if ch == '/' && i + 1 < n && chars[i + 1] == '*' {
                in_block_comment = true;
                col += 1; i += 1; // '/'
                col += 1; i += 1; // '*'
                continue;
            }

            // Enter line comment
            if ch == '-' && i + 1 < n && chars[i + 1] == '-' {
                in_line_comment = true;
                col += 1; i += 1; // first '-'
                col += 1; i += 1; // second '-'
                continue;
            }

            // Enter single-quoted string
            if ch == '\'' {
                in_single_quote = true;
                col += 1;
                i += 1;
                continue;
            }

            // Enter double-quoted identifier
            if ch == '"' {
                in_double_quote = true;
                col += 1;
                i += 1;
                continue;
            }

            // Plain SQL context — flag any non-ASCII character
            if !ch.is_ascii() {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Non-ASCII character found in SQL; use ASCII identifiers for portability"
                        .to_string(),
                    line,
                    col,
                });
            }

            // Advance position
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
        }

        diags
    }
}
