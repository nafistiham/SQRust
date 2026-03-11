use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level `sqrust.toml` structure.
///
/// # v0.1.0 — Option A (denylist)
/// All rules are enabled by default. Use `[rules] disable = [...]` to turn
/// specific rules off.
///
/// # Planned — Option B (Ruff-style allowlist, v0.2.0)
/// The fields `select` and `ignore` inside `[rules]` are reserved for a
/// future allowlist model where you opt into rule categories or individual
/// rules rather than opting out. When that lands, existing `disable` configs
/// will continue to work without changes.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub sqrust: SqrustConfig,
    #[serde(default)]
    pub rules: RulesConfig,
}

/// `[sqrust]` section — global linter settings.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct SqrustConfig {
    /// SQL dialect. Currently informational only; ANSI parser is always used.
    /// Future: "ansi" | "bigquery" | "snowflake" | "duckdb" | "postgres"
    pub dialect: Option<String>,

    /// Glob patterns for files to include. Default: all `.sql` files found
    /// by walking the given path.
    #[serde(default)]
    pub include: Vec<String>,

    /// Glob patterns for paths to exclude.
    /// Example: `["dbt_packages/**", "target/**"]`
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// `[rules]` section — rule selection.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct RulesConfig {
    /// Rules to disable. All other rules remain active.
    /// Use the full rule name: `"Convention/SelectStar"`.
    ///
    /// # Example
    /// ```toml
    /// [rules]
    /// disable = [
    ///     "Convention/SelectStar",
    ///     "Layout/LongLines",
    /// ]
    /// ```
    #[serde(default)]
    pub disable: Vec<String>,

    // ── Option B (reserved, not yet active) ─────────────────────────────────
    // The fields below are planned for v0.2.0. They are commented out so the
    // parser rejects them with a clear error rather than silently ignoring them,
    // giving users an early signal when they try to use them.
    //
    // /// Enable only these rules or categories (allowlist).
    // /// `"Convention"` enables all convention rules.
    // /// `"Convention/SelectStar"` enables one specific rule.
    // pub select: Option<Vec<String>>,
    //
    // /// Disable rules even when they appear in `select` (Ruff-style override).
    // pub ignore: Vec<String>,
}

impl Config {
    /// Load `sqrust.toml` by walking up from `start` to the filesystem root.
    /// Returns `Config::default()` (all rules enabled, no excludes) if no
    /// config file is found.
    pub fn load(start: &Path) -> Result<Self, String> {
        if let Some(path) = find_config(start) {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Invalid sqrust.toml: {}", e))
        } else {
            Ok(Config::default())
        }
    }

    /// Returns true if the rule with this name should run.
    pub fn rule_enabled(&self, name: &str) -> bool {
        !self.rules.disable.iter().any(|d| d == name)
    }
}

/// Walk up from `start` looking for `sqrust.toml`.
fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = dir.join("sqrust.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml: &str) -> Config {
        toml::from_str(toml).expect("valid toml")
    }

    #[test]
    fn empty_config_is_default() {
        let cfg = parse("");
        assert!(cfg.rules.disable.is_empty());
        assert!(cfg.sqrust.exclude.is_empty());
    }

    #[test]
    fn disable_list_parsed() {
        let cfg = parse(r#"
[rules]
disable = ["Convention/SelectStar", "Layout/LongLines"]
"#);
        assert_eq!(cfg.rules.disable.len(), 2);
        assert!(cfg.rules.disable.contains(&"Convention/SelectStar".to_string()));
    }

    #[test]
    fn rule_enabled_respects_disable() {
        let cfg = parse(r#"
[rules]
disable = ["Convention/SelectStar"]
"#);
        assert!(!cfg.rule_enabled("Convention/SelectStar"));
        assert!(cfg.rule_enabled("Layout/LongLines"));
    }

    #[test]
    fn exclude_patterns_parsed() {
        let cfg = parse(r#"
[sqrust]
exclude = ["dbt_packages/**", "target/**"]
"#);
        assert_eq!(cfg.sqrust.exclude.len(), 2);
    }

    #[test]
    fn dialect_parsed() {
        let cfg = parse(r#"
[sqrust]
dialect = "bigquery"
"#);
        assert_eq!(cfg.sqrust.dialect.as_deref(), Some("bigquery"));
    }

    #[test]
    fn unknown_field_rejected() {
        let result: Result<Config, _> = toml::from_str(r#"
[rules]
select = ["Convention"]
"#);
        assert!(result.is_err(), "select is not yet supported and should be rejected");
    }
}
