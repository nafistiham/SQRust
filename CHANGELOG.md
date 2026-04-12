# Changelog

All notable changes to SQRust are documented here.

---

## [Unreleased]

### Added
- `--dialect` flag on `check` and `fmt` — overrides `sqrust.toml` dialect per-run. Valid values: `ansi`, `bigquery`, `snowflake`, `duckdb`, `postgres`, `postgresql`, `mysql`.
- Unknown dialect values now exit with code 2 and a clear error message (previously silently fell back to GenericDialect).
- `--format json` output now includes a `severity` field (`"error"` for parse failures, `"warning"` for lint violations).
- VS Code extension (`sqrust-vscode`) — lint on save/open, Problems panel integration, `sqrust.checkFile` and `sqrust.checkWorkspace` commands.

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
