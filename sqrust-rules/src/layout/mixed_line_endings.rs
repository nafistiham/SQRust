use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MixedLineEndings;

impl Rule for MixedLineEndings {
    fn name(&self) -> &'static str {
        "Layout/MixedLineEndings"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse entirely.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let src = &ctx.source;

        let has_crlf = src.contains("\r\n");

        // A bare LF is any '\n' that is NOT preceded by '\r'.
        // We detect it by looking at consecutive byte pairs.
        let has_bare_lf = {
            let bytes = src.as_bytes();
            let mut found = false;
            for i in 0..bytes.len() {
                if bytes[i] == b'\n' {
                    let preceded_by_cr = i > 0 && bytes[i - 1] == b'\r';
                    if !preceded_by_cr {
                        found = true;
                        break;
                    }
                }
            }
            found
        };

        if has_crlf && has_bare_lf {
            vec![Diagnostic {
                rule: self.name(),
                message: "Mixed line endings detected (both CRLF and LF); normalize to one style"
                    .to_string(),
                line: 1,
                col: 1,
            }]
        } else {
            Vec::new()
        }
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let src = &ctx.source;

        let has_crlf = src.contains("\r\n");

        let has_bare_lf = {
            let bytes = src.as_bytes();
            let mut found = false;
            for i in 0..bytes.len() {
                if bytes[i] == b'\n' {
                    let preceded_by_cr = i > 0 && bytes[i - 1] == b'\r';
                    if !preceded_by_cr {
                        found = true;
                        break;
                    }
                }
            }
            found
        };

        // Only fix when there IS mixing — otherwise leave the source untouched.
        if has_crlf && has_bare_lf {
            Some(src.replace("\r\n", "\n"))
        } else {
            None
        }
    }
}
