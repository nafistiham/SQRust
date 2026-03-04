use std::path::PathBuf;

/// A single lint violation produced by a Rule.
pub struct Diagnostic {
    pub rule: &'static str,
    pub message: String,
    /// 1-indexed line number
    pub line: usize,
    /// 1-indexed column of the violation
    pub col: usize,
}

/// All information a Rule needs to check one file.
pub struct FileContext {
    pub path: PathBuf,
    pub source: String,
}

impl FileContext {
    pub fn from_source(source: &str, path: &str) -> Self {
        FileContext {
            path: PathBuf::from(path),
            source: source.to_string(),
        }
    }

    /// Returns (1-indexed line number, line content) for each line.
    pub fn lines(&self) -> impl Iterator<Item = (usize, &str)> {
        self.source.lines().enumerate().map(|(i, line)| (i + 1, line))
    }
}

/// Every lint rule implements this trait.
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic>;
    /// Returns the fixed source if this rule supports auto-fix, None otherwise.
    fn fix(&self, _ctx: &FileContext) -> Option<String> {
        None
    }
}
