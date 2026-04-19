# Migrating from sqlfluff to SQRust

## Config migration

sqlfluff uses a `.sqlfluff` INI-style config file. SQRust uses `sqrust.toml`.

**sqlfluff:**

```ini
[sqlfluff]
dialect = ansi
exclude_rules = ST07, AL05

[sqlfluff:rules:LT01]
max_line_length = 120
```

**SQRust equivalent:**

```toml
[sqrust]
# dialect is informational only in v0.1; ANSI parser is always used

[rules]
disable = [
    "Structure/WildcardInUnion",   # ST07
    "Lint/UnusedTableAlias",       # AL05
]
```

Currently, SQRust does not support per-rule configuration parameters (like `max_line_length`). This is planned for a future release.

---

## Rule name mapping

SQRust rule names follow the same concepts as sqlfluff where they overlap. The naming convention differs: SQRust uses `Category/RuleName` (e.g. `Convention/SelectStar`) while sqlfluff uses codes like `AM04`.

### Ambiguous

| sqlfluff | SQRust |
|----------|--------|
| AM01 | `Ambiguous/SelectStarWithOtherColumns` |
| AM02 | `Ambiguous/OrderByPosition` |
| AM03 | `Ambiguous/ImplicitOrderDirection` |
| AM04 | `Ambiguous/AmbiguousDateFormat` |
| AM05 | `Ambiguous/JoinWithoutCondition` |
| AM06 | `Ambiguous/FloatingPointComparison` |
| AM07 | `Ambiguous/UnionColumnMismatch` |

### Convention

| sqlfluff | SQRust |
|----------|--------|
| AL01 | `Convention/ExplicitAlias` |
| AL02 | `Convention/ExplicitColumnAlias` |
| AL03 | `Convention/ExplicitJoinType` |
| AL04 | `Convention/RedundantAlias` |
| AL05 | `Lint/UnusedTableAlias` |
| AL06 | `Ambiguous/UnaliasedExpression` |
| AL07 | `Convention/DistinctParenthesis` |
| AL08 | `Lint/DuplicateAlias` |
| AL09 | `Convention/ColonCast` |
| CV01 | `Convention/NotEqual` |
| CV02 | `Convention/SelectStar` |
| CV03 | `Convention/CommaStyle` |
| CV04 | `Convention/CountStar` |
| CV05 | `Convention/IsNull` |
| CV06 | `Convention/TrailingComma` |
| CV07 | `Convention/NoUsingClause` |
| CV08 | `Convention/BooleanComparison` |
| CV09 | `Convention/ConcatOperator` |
| CV10 | `Convention/CaseElse` |
| CV11 | `Convention/ExistsOverIn` |

### Layout

| sqlfluff | SQRust |
|----------|--------|
| LT01 | `Layout/TrailingWhitespace` |
| LT02 | `Layout/IndentationConsistency` |
| LT03 | `Layout/SpaceAfterKeyword` |
| LT04 | `Layout/LeadingComma` |
| LT05 | `Layout/LongLines` |
| LT06 | `Layout/FunctionCallSpacing` |
| LT07 | `Layout/ClauseOnNewLine` |
| LT08 | `Layout/BlankLineAfterCte` |
| LT09 | `Layout/SelectColumnPerLine` |
| LT10 | `Layout/SelectTargetNewLine` |
| LT11 | `Layout/SetOperatorNewLine` |
| LT12 | `Layout/TrailingNewline` |
| LT13 | `Layout/TrailingBlankLines` |

### Lint / Structure

| sqlfluff | SQRust |
|----------|--------|
| JJ01 | `Structure/NaturalJoin` |
| RF01 | `Lint/UnusedCte` |
| RF02 | `Lint/DuplicateJoin` |
| RF03 | `Lint/UnusedTableAlias` |
| RF04 | `Lint/ColumnAliasInWhere` |
| RF05 | `Lint/DuplicateSelectColumn` |
| RF06 | `Ambiguous/GroupByPosition` |
| ST01 | `Structure/WildcardInUnion` |
| ST02 | `Lint/DeleteWithoutWhere` |
| ST03 | `Lint/UpdateWithoutWhere` |
| ST04 | `Structure/NestedCaseInElse` |
| ST05 | `Structure/SubqueryInSelect` |
| ST06 | `Structure/SelectOnlyLiterals` |
| ST07 | `Structure/WildcardInUnion` |
| ST08 | `Structure/MixedAggregateAndColumns` |
| ST09 | `Structure/DistinctGroupBy` |

---

## Pre-commit hook

**sqlfluff:**

```yaml
repos:
  - repo: https://github.com/sqlfluff/sqlfluff
    rev: 3.2.5
    hooks:
      - id: sqlfluff-lint
        args: [--dialect, ansi]
      - id: sqlfluff-fix
        args: [--dialect, ansi]
```

**SQRust:**

```yaml
repos:
  - repo: https://github.com/nafistiham/SQRust
    rev: v0.1.4
    hooks:
      - id: sqrust
        args: [check]
```

Key differences:
- No `--dialect` flag required (ANSI is the default)
- `sqrust fmt` for auto-fix (layout rules only, in v0.1)
- Single binary — no pip install step in pre-commit

---

## Differences to be aware of

**Dialect scope.** SQRust supports ANSI, BigQuery, Snowflake, DuckDB, PostgreSQL, and MySQL parsing. Set `dialect` in `sqrust.toml` or pass `--dialect` on the CLI. Dialect-aware parsing prevents parse errors on dialect-specific syntax; dialect-specific lint rules (rules that only fire on BigQuery SQL, etc.) are on the v0.2 roadmap.

**Auto-fix scope.** sqlfluff's `--fix` covers many rule categories. SQRust's `fmt` command currently fixes layout violations only (trailing whitespace, indentation, etc.). Semantic rule fixes are on the roadmap.

**Rule parameters.** sqlfluff rules can be tuned per-project (e.g. `max_line_length = 120`). SQRust v0.1 does not support per-rule parameters — all rules use built-in defaults. This is planned for a future release.

**Rule coverage.** SQRust has 330 rules vs sqlfluff's ~89. Some SQRust rules have no sqlfluff equivalent. Some sqlfluff rules have no SQRust equivalent yet — if you depend on a specific rule, check [docs/rules.md](rules.md) or open an issue.

---

## Getting help

If you're migrating a large project and hit issues, open a GitHub issue with your `.sqlfluff` config. We can help map the rules manually.

GitHub: https://github.com/nafistiham/SQRust
