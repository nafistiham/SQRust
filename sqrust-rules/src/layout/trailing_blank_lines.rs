use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TrailingBlankLines;

impl Rule for TrailingBlankLines {
    fn name(&self) -> &'static str {
        "Layout/TrailingBlankLines"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;

        // Split into lines preserving empty trailing lines.
        // str::lines() strips the final newline, so we need a different approach.
        let raw_lines: Vec<&str> = source.split('\n').collect();

        // If the source ends with '\n', split('\n') gives us an empty string at
        // the end — that represents the end-of-file after the last newline, not
        // a blank line. We handle this by trimming that sentinel off before
        // counting trailing blanks.
        //
        // Example:
        //   "SELECT 1\n"   → ["SELECT 1", ""]      → sentinel "", 0 trailing blanks
        //   "SELECT 1\n\n" → ["SELECT 1", "", ""]  → sentinel "", 1 trailing blank ("")
        //   "SELECT 1"     → ["SELECT 1"]           → no sentinel, 0 trailing blanks

        // Total number of segments from split.
        let n = raw_lines.len();

        // The last segment is always the sentinel after a trailing '\n', OR the
        // actual last content line if the source doesn't end with '\n'.
        // We need to find the last segment with non-whitespace content, then
        // check if there are blank segments after it (excluding the sentinel).

        // Find the index of the last non-blank line.
        let last_content_idx = raw_lines
            .iter()
            .rposition(|line| !line.trim().is_empty());

        let last_content = match last_content_idx {
            None => {
                // Every segment is blank/empty — treat as empty file, no violation.
                return Vec::new();
            }
            Some(idx) => idx,
        };

        // Segments after last_content_idx that are blank (not the sentinel).
        // The sentinel is the last segment when the source ends with '\n'.
        // We want to find blank lines BETWEEN last_content and the end of file.
        //
        // Segments from (last_content + 1) up to but NOT including the sentinel
        // are trailing blank lines. The sentinel itself is just the final newline.
        //
        // If source doesn't end with '\n', n-1 is an actual content or blank line,
        // not a sentinel.
        let ends_with_newline = source.ends_with('\n');

        // How many blank-line segments exist after last_content?
        // If ends_with_newline, the last segment (index n-1) is the sentinel and
        // doesn't count as a blank line by itself.
        // Trailing blank lines = segments between (last_content+1) and the sentinel.
        let trailing_blank_count = if ends_with_newline {
            // segments at indices (last_content+1) .. (n-2) inclusive
            if last_content + 1 < n - 1 {
                n - 1 - (last_content + 1)
            } else {
                0
            }
        } else {
            // No trailing newline — segments at indices (last_content+1) .. (n-1)
            // that are blank.
            (last_content + 1..n)
                .filter(|&i| raw_lines[i].trim().is_empty())
                .count()
        };

        if trailing_blank_count == 0 {
            return Vec::new();
        }

        // The first trailing blank line is at index (last_content + 1).
        // Line number = index + 1 (1-indexed).
        let first_blank_line = last_content + 2; // (last_content + 1) + 1 for 1-indexing

        vec![Diagnostic {
            rule: self.name(),
            message: "File has trailing blank line(s)".to_string(),
            line: first_blank_line,
            col: 1,
        }]
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let diags = self.check(ctx);
        if diags.is_empty() {
            return None;
        }

        let source = &ctx.source;

        // Remove all trailing blank lines, keep a single trailing newline if
        // the original file had one.
        //
        // Strategy: find the end of the last content line and trim everything after it,
        // then append a newline.
        let trimmed = source.trim_end_matches(|c: char| c == '\n' || c == '\r' || c == ' ' || c == '\t');
        // Append exactly one newline to preserve the convention of a final newline.
        let mut result = trimmed.to_string();
        result.push('\n');
        Some(result)
    }
}
