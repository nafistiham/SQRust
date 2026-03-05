use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

use super::group_by_position::{
    match_keyword, scan_positional_list, skip_whitespace, ORDER_BY_STOP_KEYWORDS,
};

pub struct OrderByPosition;

impl Rule for OrderByPosition {
    fn name(&self) -> &'static str {
        "Ambiguous/OrderByPosition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip non-code positions (strings, comments).
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match ORDER at a word boundary.
            if let Some(after_order) = match_keyword(bytes, &skip_map, i, b"ORDER") {
                let after_ws = skip_whitespace(bytes, after_order);

                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    scan_positional_list(
                        bytes,
                        &skip_map,
                        source,
                        after_by,
                        self.name(),
                        "Avoid positional ORDER BY references; use column names",
                        ORDER_BY_STOP_KEYWORDS,
                        &mut diags,
                    );
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}
