# Changelog

All notable changes to SQRust are documented here.

---

## [Unreleased]

### Breaking
- **18 rules gained the `Category/` prefix their documentation already used.** `TooManyJoins` is now `Structure/TooManyJoins`, `ParenthesisSpacing` is now `Layout/ParenthesisSpacing`, and so on. Previously these rules returned a bare name, so the documented `disable = ["Structure/TooManyJoins"]` silently did nothing and `--category` reached only 312 of 330 rules. If your `sqrust.toml` disables any of the following by bare name, add the prefix: `CaseWhenCount`, `ExcessiveGroupByColumns`, `ExcessiveWhereConditions`, `FunctionCallDepth`, `HavingWithoutAggregate`, `InSingleValue`, `LargeInList`, `MaxBlankLines`, `NaturalJoin`, `NoSelectAll`, `OrderByInSubquery`, `ParenthesisSpacing`, `SelectDistinctStar`, `TooManyCtes`, `TooManyJoins`, `TooManyUnions`, `UnnecessaryElseNull`, `WindowWithoutOrderBy`.

### Fixed
- **`sqrust fmt` no longer corrupts files.** Three separate data-loss bugs, each of which could rewrite a file that had no violations at all:
  - `Convention/NotEqual` panicked (exit 101) on any file containing two or more string literals or comments — i.e. most real SQL.
  - `Layout/WhitespaceBeforeSemicolon` mangled multi-byte UTF-8, turning `café` into `cafÃ©`.
  - `Layout/TrailingWhitespace` and `Layout/MaxBlankLines` silently rewrote CRLF files as LF.
- `fmt` now verifies the formatted output still parses before writing, and skips the file if it would not.
- `fmt` warns when formatting a file that could not be parsed (e.g. a dbt Jinja model), since only text-level rules apply and the parse check cannot run.
- `sqrust.toml` is now found when the linted path is relative — `sqrust check .` from a subdirectory previously ignored the project config entirely.
- `sqrust rules --enable/--disable` no longer deletes the comments and formatting in your `sqrust.toml`, and no longer rewrites the file when nothing changed.
- A malformed `sqrust.toml` (for example `disable = "x"` instead of an array) now reports a clear error and exits 2 instead of panicking.
- `check` and `fmt` exit non-zero when a file cannot be read or written, instead of reporting success.
- `--format` is validated; `--format jsonl` previously produced text output with no warning.
- Duplicate paths (`sqrust check . .`) no longer double-report every violation.
- `Structure/NestedSubquery` measures real nesting depth. It previously counted every `(SELECT` in the file without ever decrementing, so three independent statements were reported as "nesting depth 3", and any dbt model with three or more CTEs was flagged.
- `Layout/ClauseOnNewLine` no longer flags a correctly formatted `LEFT JOIN`, SQL keywords inside comments, or `ORDER BY` within an `OVER (...)` window specification.
- `Layout/SelectColumnPerLine` and `Layout/GroupByColumnPerLine` no longer treat a function's argument separator as a column separator — `COALESCE(a, b)` and `date_trunc('month', ts)` were flagged as "multiple columns on one line".
- `Layout/ParenthesisSpacing` no longer flags the indentation before a `)` that closes a multi-line expression on its own line. The same detection drove its `fix()`, so `sqrust fmt` would have dedented every multi-line function call and subquery.
- `Layout/FunctionCallSpacing` no longer treats `JOIN ... USING (col)` as a function call with a stray space. `VALUES`, `RETURNING`, `INTO`, `ALL`, `ANY`, `SOME` and `LATERAL` are also recognised as syntax.
- `Layout/ArithmeticOperatorAtLineEnd` no longer flags the trailing `-` of dbt/Jinja whitespace-trim tags (`{#-`, `{%-`, `{{-`).
- `Layout/ArithmeticOperatorPadding` no longer treats the wildcard in `SELECT *, other_col` or `t.*` as unpadded multiplication.

Together these cut violations on the vendored dbt corpus from 516 to 445; excluding the Capitalisation rules (a style preference, not a defect) the remaining findings across those 12 files total 83, or 71 once the 12 expected Jinja parse errors are also set aside.

### Fixed (docs and packaging)
- `sqrust check` output is now sorted by file, then line, then column — it previously arrived in rule-registry and directory-entry order, so the documented "sorted by file path then line number" was not what users saw. JSON output is sorted the same way.
- `docs/rules.md` listed 298 of the 330 rules — the entire Wave 32–34 batch was missing. All 330 rules are now documented.
- README example output now quotes the real messages emitted by `TrailingWhitespace`, `ColonCast`, and `LongLines` instead of paraphrases.
- The sqlfluff rule count quoted in the README, migration guide, and benchmark script was corrected from ~89 to 73 (as reported by `sqlfluff rules` on v4.1.0); sqruff's was confirmed as exactly 62.
- The VS Code command is titled `SQRust: Check File`, matching what the READMEs and extension changelog already claimed.
- Release builds now include `aarch64-unknown-linux-gnu`, so the one-line installer's Linux/ARM64 path downloads a real artifact instead of a 404.
- Benchmark numbers were removed from `docs/architecture.md`; the README is the single source of truth for them (as `docs/product.md` already claimed).
- Every rule now has at least 13 tests; five early rules (`TrailingWhitespace`, `TrailingNewline`, `CommaStyle`, `TabIndentation`, `LongLines`) had 8–12.

### Added
- `sqrust fmt --check` reports which files would be reformatted without writing them, and exits 1 if any would. Intended for CI.

### Performance
- `sqrust fmt` is roughly 100× faster: it parsed the file once per rule (about 330 full parses per file) and ran single-threaded. It now parses once, re-parses only when a rule actually changes the source, and runs in parallel like `check`.
- Directory walking skips hidden directories and prunes directories excluded by a `dir/**` pattern instead of traversing them and filtering afterwards.

---

## [0.1.4] — 2026-04-14

### Added
- `--dialect` flag on `check` and `fmt` — overrides `sqrust.toml` dialect per-run. Valid values: `ansi`, `bigquery`, `snowflake`, `duckdb`, `postgres`, `postgresql`, `mysql`.
- Unknown dialect values now exit with code 2 and a clear error message (previously silently fell back to GenericDialect).
- `--format json` output now includes a `severity` field (`"error"` for parse failures, `"warning"` for lint violations).
- VS Code extension — lint on save/open, Problems panel integration, `sqrust.checkFile` and `sqrust.checkWorkspace` commands, dialect setting. Available on the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=NafisTiham.sqrust).
- **330 rules** (up from 300): 30 new rules across Convention, Layout, Lint, Structure, and Ambiguous categories.

---

## [0.1.3] — 2026-03-28

### Fixed
- `sqrust check` now reports parse errors as `[Parse/Error]` diagnostics and exits nonzero — invalid SQL no longer silently passes CI
- `sqrust fmt` now applies all fixers in a composing pipeline (each fixer sees the output of the previous), preventing later fixes from overwriting earlier ones
- `sqrust rules --disable/--enable` now validates the rule name against the registry and exits nonzero on typos
- `[sqrust] include = [...]` in `sqrust.toml` is now applied during file filtering (was parsed but ignored)

---

## [0.1.2] — 2026-03-25

### Added
- **Dialect support** — set `dialect = "bigquery"` (or `snowflake`, `duckdb`, `postgres`, `mysql`, `ansi`) in `sqrust.toml` to use a dialect-aware SQL parser. Dialect is passed directly to sqlparser-rs.
- **Jinja/dbt template awareness** — text-scan rules (layout, spacing) now skip `{{ }}`, `{% %}`, and `{# #}` blocks, eliminating false positives on dbt model files.

### Fixed
- `Layout/SpaceAroundEquals` and other text-scan rules no longer fire on content inside Jinja template blocks (e.g. `{{ dbt_date.get_base_dates(n_dateparts=365*10) }}`).

---

## [0.1.1] — 2026-03-12

### Added
- `sqrust rules` subcommand — browse all 300 rules with enabled/disabled status
- `sqrust rules --disable <Rule>` — disable a rule and write it to `sqrust.toml` automatically
- `sqrust rules --enable <Rule>` — re-enable a disabled rule
- `sqrust rules --category <Category>` — filter rules by category
- `sqrust check --format json` — structured JSON output for CI integration
- `toml` dependency wired into `sqrust-cli` for config read/write in `rules` subcommand

### Rules added (Wave 19–31, ~125 new rules)
- **Ambiguous:** `AddMonthsFunction`, `AmbiguousBoolOp`, `CastToVarchar`, `ChainedComparisons`, `ConcatFunctionNullArg`, `ConvertFunction`, `DateArithmetic`, `DateTruncFunction`, `DateaddFunction`, `DistinctWithWindowFunction`, `DivisionByZero`, `ExistsSelectList`, `FloatingPointComparison`, `FormatFunction`, `FullOuterJoin`, `FunctionOnFilteredColumn`, `ImplicitBooleanComparison`, `InSubqueryMultiColumn`, `InconsistentColumnReference`, `InconsistentOrderByDirection`, `IntegerDivision`, `IntervalExpression`, `MixedJoinTypes`, `MultipleCountDistinct`, `NonDeterministicGroupBy`, `NullSafeEquality`, `NullsOrdering`, `OrInJoinCondition`, `RegexpFunction`, `SelectDistinctOrderBy`, `SelectDistinctWithGroupBy`, `SelectNullExpression`, `SelfComparison`, `SelfJoin`, `StringToNumberComparison`, `SubqueryInGroupBy`, `SubqueryInOrderBy`, `SubstringFunction`, `UnsafeDivision`, `WindowFunctionWithoutPartition`, `YearMonthDayFunction`
- **Capitalisation:** `Literals`, `Types`
- **Convention:** `AvoidIif`, `CastVsConvert`, `CoalesceNullArg`, `CommaStyle` (extended), `ConcatOperator`, `ExistsOverIn`, `ExplicitAlias`, `ExplicitColumnAlias`, `ExplicitJoinType`, `GetDate`, `IfNullFunction`, `InNullComparison`, `InSingleValue`, `JoinConditionStyle`, `LeadingZeroNumeric`, `LeftJoin`, `LenFunction`, `LikeTautology`, `LikeWithoutWildcard`, `NStringLiteral`, `NegatedNotLike`, `NoCharType` (extended), `NoCharindexFunction`, `NoCurrentTimestampInWhere`, `NoDecodeFunction`, `NoDualTable`, `NoIFFunction`, `NoIsnullFunction`, `NoMinusOperator`, `NoNullDefault`, `NoNvl2`, `NoRownum`, `NoSelectAll`, `NoSysdate`, `NoUsingClause`, `NoValuesFunction`, `NullableConcat`, `NvlFunction`, `OrInsteadOfIn`, `OrderByWithOffset`, `PivotUnpivot`, `PreferExtract`, `RedundantAlias`, `SelectDistinctStar`, `SelectTopN`, `StringAggSeparator`, `TopNWithoutOrder`, `TryCast`, `UnnecessaryCaseWhen`, `UpperLower`, `UseCurrentDate`
- **Layout:** `AliasOnNewLine`, `ArithmeticOperatorAtLineEnd`, `ArithmeticOperatorPadding`, `BlankLineAfterCte`, `BlankLineBetweenCTEs`, `BlankLineBetweenStatements`, `ClauseOnNewLine` (extended), `ClosingParenNewLine`, `CommaAfterLastColumn`, `CommentStyle`, `ComparisonOperatorSpacing`, `ConsistentCommentStyle`, `ConsistentQuoteStyle`, `FunctionCallSpacing`, `GroupByOnNewLine`, `HavingOnNewLine`, `IndentationConsistency`, `JoinOnNewLine`, `LeadingOperator`, `LimitOnNewLine`, `MaxIdentifierLength`, `MaxLineCount`, `MaxStatementLength`, `MixedLineEndings`, `NestedParentheses`, `NoMultipleStatementsOnLine`, `NoSpaceAfterUnaryMinus`, `NoSpaceAroundDot`, `NoSpaceBeforeOpenParen`, `NoSpaceInsideBrackets`, `OperatorAtLineStart`, `OrderByOnNewLine`, `SelectColumnPerLine`, `SelectStarSpacing`, `SelectTargetNewLine`, `SetOperatorNewLine`, `SpaceAfterAs`, `SpaceAfterNot`, `SpaceAfterSemicolon`, `SpaceAroundConcatOperator`, `SpaceBeforeIn`, `TabIndentation` (extended), `TrailingBlankLines`, `UnicodeIdentifiers`, `UnnecessaryAliasQuoting`, `WhereOnNewLine`, `WhitespaceBeforeSemicolon`
- **Lint:** `AddColumnWithoutDefault`, `AlterColumnType`, `AlterTableAddNotNullWithoutDefault`, `AlterTableDropColumn`, `AlterTableRenameColumn`, `AlterTableSetNotNull`, `CommentWithoutSpace`, `ConsecutiveSemicolons`, `CreateIndexIfNotExists`, `CreateOrReplace`, `CreateSchemaStatement`, `CreateSequenceStatement`, `CreateTableWithoutPrimaryKey`, `CreateTempTable`, `CreateViewWithSelectStar`, `CrossDatabaseReference`, `DropColumnIfExists`, `DropIndex`, `DropSchemaStatement`, `DropTableIfExists`, `DropViewIfExists`, `DuplicateColumnInCreate`, `DuplicateCondition`, `DuplicateCteNames`, `EmptyInList`, `EmptyStringComparison`, `ExecuteStatement`, `GrantAllPrivileges`, `InsertIgnore`, `InsertOrReplace`, `InsertOverwrite`, `KeywordIdentifier`, `MergeStatement`, `MultiplePrimaryKeys`, `NonDeterministicFunction`, `NullInNotIn`, `OnConflictClause`, `OrderByInView`, `RecursiveCte`, `SelectForUpdate`, `SelectIntoTable`, `SelectWithoutFrom`, `SetVariableStatement`, `TruncateTable`, `UnusedTableAlias`, `UpdateSetDuplicate`, `WhereTautology`
- **Structure:** `AggregateInWhere`, `AggregateStar`, `AntiJoinPattern`, `CaseWhenCount`, `CorrelatedSubquery`, `CountDistinctInGroup`, `CrossApply`, `DeepCteChain`, `ExceptAll`, `ExcessiveGroupByColumns`, `ExcessiveUnionChain`, `ExcessiveWhereConditions`, `FunctionCallDepth`, `HavingConditionsCount`, `HavingWithoutAggregate`, `HavingWithoutSelectAgg`, `InsertSelectStar`, `InsertValuesLimit`, `LargeOffset`, `LateralColumnAlias`, `LateralJoin`, `MaxJoinOnConditions`, `MaxSelectColumns`, `MixedAggregateAndColumns`, `NestedAggregate`, `NestedCaseInElse`, `NestedSubquery`, `OrderByInSubquery`, `ScalarSubqueryInSelect`, `SelectOnlyLiterals`, `SelectStarInCTE`, `SetOpPrecedence`, `SubqueryInHaving`, `SubqueryInJoinCondition`, `TooManyOrderByColumns`, `TooManySubqueries`, `TooManyWindowFunctions`, `UnionBranchLimit`, `UnusedJoin`, `UpdateWithJoin`, `WindowFrameAllRows`, `WindowFrameFullPartition`, `WindowFunctionInWhere`, `ZeroLimitClause`

### Fixed
- Security: resolved path traversal risk in file walker
- Config `is_excluded()` now correctly matches `dbt_packages/**` regardless of working directory

---

## [0.1.0] — 2026-02-28

### Initial release

- 175 rules across 6 categories: Convention, Layout, Lint, Structure, Ambiguous, Capitalisation
- `sqrust check <path>` — lint SQL files
- `sqrust fmt <path>` — auto-fix layout violations
- `sqrust.toml` config — `exclude` globs and `disable` rule list
- Config auto-discovery: walks up from linted path
- File-level parallelism via `rayon`
- Pre-built binaries: macOS arm64, macOS x86_64, Linux x86_64, Windows x86_64
- Published to crates.io: `sqrust-core`, `sqrust-rules`, `sqrust-cli`
