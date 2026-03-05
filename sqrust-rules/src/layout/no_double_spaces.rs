use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoDoubleSpaces;

impl Rule for NoDoubleSpaces {
    fn name(&self) -> &'static str {
        "Layout/NoDoubleSpaces"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();

        if len == 0 {
            return None;
        }

        let skip = build_skip_set(bytes, len);
        let mut result: Vec<u8> = Vec::with_capacity(len);
        let mut changed = false;
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b == b' ' {
                let run_start = i;
                let mut run_end = i;
                while run_end < len && bytes[run_end] == b' ' {
                    run_end += 1;
                }
                let run_len = run_end - run_start;

                let is_indent = is_line_start_run(bytes, run_start);

                if run_len >= 2 && !is_indent {
                    // Collapse only if at least one byte in the run is code
                    // (outside all skip regions). Runs entirely inside strings
                    // or comments are preserved as-is.
                    let any_code = (run_start..run_end).any(|p| !skip[p]);

                    if any_code {
                        result.push(b' ');
                        changed = true;
                        i = run_end;
                        continue;
                    }
                }

                // Emit as-is.
                for p in run_start..run_end {
                    result.push(bytes[p]);
                }
                i = run_end;
            } else {
                result.push(b);
                i += 1;
            }
        }

        if !changed {
            return None;
        }

        Some(String::from_utf8(result).expect("source was valid UTF-8"))
    }
}

/// Scan `source` and return a Diagnostic for every run of 2+ consecutive spaces
/// that is outside skip regions and is not at the start of a line.
/// A run of N >= 2 spaces produces exactly one violation at the first space.
fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);

    let mut diags = Vec::new();
    let mut i = 0;
    let mut line: usize = 1;
    let mut line_start: usize = 0;

    while i < len {
        let b = bytes[i];

        if b == b' ' {
            let run_start = i;
            let run_start_col = run_start - line_start + 1;

            let mut run_end = i;
            while run_end < len && bytes[run_end] == b' ' {
                run_end += 1;
            }
            let run_len = run_end - run_start;

            if run_len >= 2 {
                let is_indent = is_line_start_run(bytes, run_start);

                if !is_indent {
                    let any_code = (run_start..run_end).any(|p| !skip[p]);

                    if any_code {
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: "Multiple consecutive spaces found; use a single space"
                                .to_string(),
                            line,
                            col: run_start_col,
                        });
                    }
                }
            }

            // Advance past the run (no newlines inside a space run).
            i = run_end;
            continue;
        }

        if b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
        i += 1;
    }

    diags
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}

/// Returns `true` if `run_start` is preceded only by spaces/tabs since
/// the last newline (or the start of the file) — i.e., this run is indentation.
fn is_line_start_run(bytes: &[u8], run_start: usize) -> bool {
    if run_start == 0 {
        return true;
    }
    let mut j = run_start;
    while j > 0 {
        j -= 1;
        if bytes[j] == b'\n' {
            return true;
        }
        if bytes[j] != b' ' && bytes[j] != b'\t' {
            return false;
        }
    }
    // Reached start of file with only spaces/tabs — first-line indentation.
    true
}
