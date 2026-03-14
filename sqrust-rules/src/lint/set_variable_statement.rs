use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct SetVariableStatement;

impl Rule for SetVariableStatement {
    fn name(&self) -> &'static str {
        "Lint/SetVariableStatement"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let mut diags = Vec::new();

        let lower = source.to_lowercase();
        let bytes = lower.as_bytes();
        let src_bytes = source.as_bytes();
        let len = bytes.len();
        let keyword = b"set";
        let kw_len = keyword.len();

        let mut i = 0;
        while i + kw_len <= len {
            if !skip.contains(&i) && bytes[i..i + kw_len] == *keyword {
                let before_ok = i == 0
                    || {
                        let b = bytes[i - 1];
                        !b.is_ascii_alphanumeric() && b != b'_'
                    };
                let after_pos = i + kw_len;
                let after_ok = after_pos >= len
                    || {
                        let b = bytes[after_pos];
                        !b.is_ascii_alphanumeric() && b != b'_'
                    };

                if before_ok && after_ok {
                    // Skip whitespace after SET keyword
                    let mut j = after_pos;
                    while j < len
                        && (src_bytes[j] == b' '
                            || src_bytes[j] == b'\t'
                            || src_bytes[j] == b'\r'
                            || src_bytes[j] == b'\n')
                    {
                        j += 1;
                    }
                    // Check if next non-whitespace character is '@'
                    if j < len && src_bytes[j] == b'@' {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "SET @variable is a dialect-specific variable assignment \
                                      (MySQL/SQL Server); not supported in standard SQL or \
                                      analytical databases"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
            i += 1;
        }

        diags
    }
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
