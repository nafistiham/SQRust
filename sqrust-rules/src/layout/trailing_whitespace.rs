use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TrailingWhitespace;

impl Rule for TrailingWhitespace {
    fn name(&self) -> &'static str {
        "Layout/TrailingWhitespace"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for (line_num, line) in ctx.lines() {
            let trimmed = line.trim_end();
            if trimmed.len() < line.len() {
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Trailing whitespace".to_string(),
                    line: line_num,
                    col: trimmed.len() + 1,
                });
            }
        }
        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        // Walk the source terminator-by-terminator rather than using
        // `lines()` + `join("\n")`: that pair silently rewrites CRLF files as
        // LF, changing every line of a Windows-authored file as a side effect
        // of trimming spaces.
        let source = &ctx.source;
        let mut fixed = String::with_capacity(source.len());
        let mut rest = source.as_str();

        loop {
            match rest.find('\n') {
                Some(nl) => {
                    let (chunk, tail) = rest.split_at(nl + 1);
                    let content = &chunk[..chunk.len() - 1];
                    let (body, cr) = match content.strip_suffix('\r') {
                        Some(b) => (b, "\r"),
                        None => (content, ""),
                    };
                    fixed.push_str(trim_spaces_tabs(body));
                    fixed.push_str(cr);
                    fixed.push('\n');
                    rest = tail;
                }
                None => {
                    // Final segment, no trailing newline.
                    fixed.push_str(trim_spaces_tabs(rest));
                    break;
                }
            }
        }

        // Only report a fix when something actually changed — a rule that
        // always returns Some rewrites files that have no violations.
        if fixed == *source {
            None
        } else {
            Some(fixed)
        }
    }
}

fn trim_spaces_tabs(s: &str) -> &str {
    s.trim_end_matches(|c| c == ' ' || c == '\t')
}
