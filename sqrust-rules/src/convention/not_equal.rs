use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NotEqual;

/// Iterates over characters in `source` and calls `visitor` for each character
/// that is NOT inside a single-quoted string, double-quoted identifier,
/// line comment (`--`), or block comment (`/* ... */`).
///
/// `visitor` receives (byte_offset, char, 1-indexed line, 1-indexed col).
fn visit_outside_tokens<F>(source: &str, mut visitor: F)
where
    F: FnMut(usize, char, usize, usize),
{
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut line = 1usize;
    let mut col = 1usize;
    // Compute byte offsets per char position
    let byte_offsets: Vec<usize> = {
        let mut offs = Vec::with_capacity(len);
        let mut off = 0;
        for ch in &chars {
            offs.push(off);
            off += ch.len_utf8();
        }
        offs
    };

    while i < len {
        let ch = chars[i];

        // Line comment: -- to end of line
        if ch == '-' && i + 1 < len && chars[i + 1] == '-' {
            // skip to end of line
            while i < len && chars[i] != '\n' {
                if chars[i] == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            col += 2;
            while i < len {
                if chars[i] == '\n' {
                    line += 1;
                    col = 1;
                    i += 1;
                } else if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                    i += 2;
                    col += 2;
                    break;
                } else {
                    col += 1;
                    i += 1;
                }
            }
            continue;
        }

        // Single-quoted string: '...' ('' is escape)
        if ch == '\'' {
            col += 1;
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    // check for escaped quote ''
                    if i + 1 < len && chars[i + 1] == '\'' {
                        col += 2;
                        i += 2;
                    } else {
                        col += 1;
                        i += 1;
                        break;
                    }
                } else if chars[i] == '\n' {
                    line += 1;
                    col = 1;
                    i += 1;
                } else {
                    col += 1;
                    i += 1;
                }
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if ch == '"' {
            col += 1;
            i += 1;
            while i < len {
                if chars[i] == '"' {
                    col += 1;
                    i += 1;
                    break;
                } else if chars[i] == '\n' {
                    line += 1;
                    col = 1;
                    i += 1;
                } else {
                    col += 1;
                    i += 1;
                }
            }
            continue;
        }

        // Normal character — call visitor
        let byte_off = byte_offsets[i];
        visitor(byte_off, ch, line, col);

        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }
}

impl Rule for NotEqual {
    fn name(&self) -> &'static str {
        "Convention/NotEqual"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let source = &ctx.source;

        // Collect positions of '!' that are outside strings/comments and followed by '='
        visit_outside_tokens(source, |byte_off, ch, line, col| {
            if ch == '!' {
                // Check the next byte character
                let rest = &source[byte_off..];
                if rest.starts_with("!=") {
                    diags.push(Diagnostic {
                        rule: "Convention/NotEqual",
                        message: "Use '<>' instead of '!=' for ANSI SQL compatibility".to_string(),
                        line,
                        col,
                    });
                }
            }
        });

        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();

        // Build a set of byte offsets that are inside strings/comments
        let mut skip_ranges: Vec<(usize, usize)> = Vec::new();

        let byte_offsets: Vec<usize> = {
            let mut offs = Vec::with_capacity(len);
            let mut off = 0;
            for ch in &chars {
                offs.push(off);
                off += ch.len_utf8();
            }
            offs
        };

        let mut i = 0;
        while i < len {
            let ch = chars[i];

            if ch == '-' && i + 1 < len && chars[i + 1] == '-' {
                let start = byte_offsets[i];
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                let end = if i < len { byte_offsets[i] } else { source.len() };
                skip_ranges.push((start, end));
                continue;
            }

            if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
                let start = byte_offsets[i];
                i += 2;
                while i < len {
                    if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                let end = if i < len { byte_offsets[i] } else { source.len() };
                skip_ranges.push((start, end));
                continue;
            }

            if ch == '\'' {
                let start = byte_offsets[i];
                i += 1;
                while i < len {
                    if chars[i] == '\'' {
                        if i + 1 < len && chars[i + 1] == '\'' {
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                let end = if i < len { byte_offsets[i] } else { source.len() };
                skip_ranges.push((start, end));
                continue;
            }

            if ch == '"' {
                let start = byte_offsets[i];
                i += 1;
                while i < len {
                    if chars[i] == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let end = if i < len { byte_offsets[i] } else { source.len() };
                skip_ranges.push((start, end));
                continue;
            }

            i += 1;
        }

        // Replace `!=` occurrences that are NOT inside any skip range
        let source_bytes = source.as_bytes();
        let src_len = source.len();
        let mut result = String::with_capacity(src_len);
        let mut pos = 0;

        while pos < src_len {
            // Check if this position is inside a skip range
            let in_skip = skip_ranges.iter().any(|&(s, e)| pos >= s && pos < e);
            if in_skip {
                // Find the range end and copy verbatim
                let range_end = skip_ranges
                    .iter()
                    .filter(|&&(s, _)| pos >= s)
                    .map(|&(_, e)| e)
                    .min()
                    .unwrap_or(pos + 1);
                result.push_str(&source[pos..range_end]);
                pos = range_end;
                continue;
            }

            if pos + 1 < src_len && source_bytes[pos] == b'!' && source_bytes[pos + 1] == b'=' {
                result.push_str("<>");
                pos += 2;
            } else {
                let ch = source[pos..].chars().next().unwrap();
                result.push(ch);
                pos += ch.len_utf8();
            }
        }

        Some(result)
    }
}
