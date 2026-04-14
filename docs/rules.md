# SQRust Rule Catalog

330 rules across 6 categories. All rules are enabled by default.

Disable rules via `sqrust.toml`:

```toml
[rules]
disable = ["Convention/SelectStar", "Layout/LongLines"]
```

Or from the CLI:

```bash
sqrust rules --disable Convention/SelectStar
sqrust rules --enable Convention/SelectStar
```

---

## Ambiguous (65 rules)

Rules that flag SQL which is syntactically valid but semantically unclear or dialect-dependent.

| Rule | Description |
|------|-------------|
| `Ambiguous/AddMonthsFunction` | `ADD_MONTHS` behavior differs across dialects |
| `Ambiguous/AmbiguousBoolOp` | Complex boolean expressions without explicit parentheses |
| `Ambiguous/AmbiguousDateFormat` | Date literals that parse differently by locale or dialect |
| `Ambiguous/BetweenNullBoundary` | `BETWEEN` with a NULL boundary always evaluates to UNKNOWN |
| `Ambiguous/CaseNullCheck` | `CASE` branches checking NULL without `IS NULL` |
| `Ambiguous/CastToVarchar` | `CAST(... AS VARCHAR)` without a length |
| `Ambiguous/CastWithoutLength` | `CAST` to a character type without specifying length |
| `Ambiguous/ChainedComparisons` | Chained comparisons like `a < b < c` are not transitive in SQL |
| `Ambiguous/CoalesceWithSingleArg` | `COALESCE` with a single argument is a no-op |
| `Ambiguous/ColumnNameConflict` | Column name conflicts with a SQL keyword or function |
| `Ambiguous/ConcatFunctionNullArg` | `CONCAT` propagates or ignores NULLs depending on dialect |
| `Ambiguous/ConvertFunction` | `CONVERT` behavior is dialect-specific |
| `Ambiguous/CrossJoinKeyword` | Implicit cross join via comma instead of explicit `CROSS JOIN` |
| `Ambiguous/DateArithmetic` | Date arithmetic with plain integers (dialect-specific units) |
| `Ambiguous/DateaddFunction` | `DATEADD` is not standard SQL |
| `Ambiguous/DateTruncFunction` | `DATE_TRUNC` truncation unit behavior differs by dialect |
| `Ambiguous/DistinctWithWindowFunction` | `DISTINCT` combined with a window function has unclear semantics |
| `Ambiguous/DivisionByZero` | Integer division by a literal zero |
| `Ambiguous/ExistsSelectList` | `EXISTS (SELECT col ...)` — the select list is irrelevant |
| `Ambiguous/FloatingPointComparison` | Equality comparison on floating-point columns |
| `Ambiguous/FormatFunction` | `FORMAT` function behavior differs across databases |
| `Ambiguous/FullOuterJoin` | `FULL OUTER JOIN` support varies; make intention explicit |
| `Ambiguous/FunctionOnFilteredColumn` | Function wrapping a filtered column prevents index use |
| `Ambiguous/GroupByPosition` | `GROUP BY 1` uses positional reference, not column name |
| `Ambiguous/HavingWithoutGroupBy` | `HAVING` clause without `GROUP BY` |
| `Ambiguous/ImplicitBooleanComparison` | Comparing boolean to `TRUE`/`FALSE` explicitly |
| `Ambiguous/ImplicitCrossJoin` | Comma-separated tables in `FROM` without `WHERE` join condition |
| `Ambiguous/ImplicitOrderDirection` | `ORDER BY` column without explicit `ASC` or `DESC` |
| `Ambiguous/InSubqueryMultiColumn` | `IN (subquery)` returning multiple columns |
| `Ambiguous/InconsistentColumnReference` | Same column referenced inconsistently (qualified vs unqualified) |
| `Ambiguous/InconsistentOrderByDirection` | Mixed `ASC`/`DESC` without clear pattern |
| `Ambiguous/IntegerDivision` | Integer division truncates — may be unintentional |
| `Ambiguous/IntervalExpression` | Interval literals have dialect-specific syntax |
| `Ambiguous/JoinWithoutCondition` | JOIN without any ON or USING condition |
| `Ambiguous/MixedJoinTypes` | Mixing `INNER JOIN` and comma-separated tables |
| `Ambiguous/MultipleCountDistinct` | Multiple `COUNT(DISTINCT ...)` in one query |
| `Ambiguous/NonDeterministicGroupBy` | Non-grouped column in SELECT without aggregate |
| `Ambiguous/NullSafeEquality` | `<=>` operator is not standard SQL |
| `Ambiguous/NullsOrdering` | NULL sort order (`NULLS FIRST`/`LAST`) defaults differ by dialect |
| `Ambiguous/OrInJoinCondition` | `OR` in a JOIN condition can cause unexpected row multiplication |
| `Ambiguous/OrderByPosition` | `ORDER BY 1` uses positional reference, not column name |
| `Ambiguous/RedundantBetween` | `BETWEEN x AND x` is equivalent to equality |
| `Ambiguous/RegexpFunction` | `REGEXP` / `RLIKE` behavior differs across dialects |
| `Ambiguous/SelectDistinctOrderBy` | `SELECT DISTINCT` with `ORDER BY` on a non-selected column |
| `Ambiguous/SelectDistinctWithGroupBy` | `DISTINCT` combined with `GROUP BY` is redundant or confusing |
| `Ambiguous/SelectNullExpression` | Selecting a bare NULL without an alias |
| `Ambiguous/SelectStarWithOtherColumns` | `SELECT *, col` — column ordering is non-deterministic |
| `Ambiguous/SelfComparison` | Column compared to itself (e.g. `a = a`) |
| `Ambiguous/SelfJoin` | Table joined to itself without aliasing |
| `Ambiguous/StringToNumberComparison` | Implicit string-to-number coercion in a comparison |
| `Ambiguous/SubqueryInGroupBy` | Subquery in `GROUP BY` clause |
| `Ambiguous/SubqueryInOrderBy` | Subquery in `ORDER BY` clause |
| `Ambiguous/SubstringFunction` | `SUBSTR` / `SUBSTRING` argument order differs by dialect |
| `Ambiguous/TableAliasConflict` | Table alias conflicts with another table name in scope |
| `Ambiguous/UnaliasedExpression` | Expression in SELECT without an alias |
| `Ambiguous/UnionColumnMismatch` | UNION branches have different column counts |
| `Ambiguous/UnsafeDivision` | Division where the denominator may be NULL or zero |
| `Ambiguous/WindowFunctionWithoutPartition` | Window function without `PARTITION BY` operates over all rows |
| `Ambiguous/YearMonthDayFunction` | `YEAR()`, `MONTH()`, `DAY()` are not standard SQL |

---

## Capitalisation (4 rules)

Rules that enforce consistent casing for SQL elements.

| Rule | Description |
|------|-------------|
| `Capitalisation/Functions` | Built-in function names should be uppercase |
| `Capitalisation/Keywords` | SQL keywords should be uppercase |
| `Capitalisation/Literals` | Boolean and NULL literals should be uppercase |
| `Capitalisation/Types` | Data type names should be uppercase |

---

## Convention (69 rules)

Rules that enforce style and portability conventions.

| Rule | Description |
|------|-------------|
| `Convention/AvoidIif` | `IIF` is a SQL Server extension; use `CASE WHEN` |
| `Convention/BooleanComparison` | Avoid `WHERE flag = TRUE`; use `WHERE flag` |
| `Convention/CaseElse` | `CASE` expression without an `ELSE` branch |
| `Convention/CastVsConvert` | Prefer `CAST` over `CONVERT` for portability |
| `Convention/Coalesce` | Prefer `COALESCE` over `ISNULL`/`NVL`/`IFNULL` |
| `Convention/CoalesceNullArg` | `COALESCE` argument is a literal NULL |
| `Convention/ColonCast` | PostgreSQL `::` cast syntax; use `CAST()` for portability |
| `Convention/CommaStyle` | Commas should be at the end of lines, not the start |
| `Convention/ConcatOperator` | Use `||` or `CONCAT()` consistently |
| `Convention/CountStar` | Use `COUNT(*)` not `COUNT(1)` |
| `Convention/DistinctParenthesis` | `DISTINCT` is not a function; remove parentheses |
| `Convention/ExistsOverIn` | Prefer `EXISTS` over `IN (subquery)` for readability |
| `Convention/ExplicitAlias` | Columns and tables should have explicit aliases |
| `Convention/ExplicitColumnAlias` | Column expressions should have explicit aliases |
| `Convention/ExplicitJoinType` | `JOIN` without `INNER`/`LEFT`/`RIGHT`/`CROSS` qualifier |
| `Convention/GetDate` | `GETDATE()` is SQL Server–specific; use `CURRENT_TIMESTAMP` |
| `Convention/IfNullFunction` | `IFNULL` is dialect-specific; use `COALESCE` |
| `Convention/InNullComparison` | `col IN (NULL)` never matches; use `IS NULL` |
| `Convention/InSingleValue` | `IN (single_value)` should be rewritten as `=` |
| `Convention/IsNull` | Use `IS NULL` not `= NULL` |
| `Convention/JoinConditionStyle` | JOIN conditions should use `ON` not `USING` |
| `Convention/LeadingZeroNumeric` | Numeric literals like `.5` should be `0.5` |
| `Convention/LeftJoin` | `LEFT OUTER JOIN` should be written as `LEFT JOIN` |
| `Convention/LenFunction` | `LEN()` is SQL Server–specific; use `LENGTH()` |
| `Convention/LikePercentOnly` | `LIKE '%'` matches everything; use `IS NOT NULL` instead |
| `Convention/LikeTautology` | `LIKE 'exact_string'` without wildcards is equivalent to `=` |
| `Convention/LikeWithoutWildcard` | `LIKE` pattern contains no wildcard characters |
| `Convention/NStringLiteral` | `N'...'` Unicode prefix is SQL Server–specific |
| `Convention/NegatedNotLike` | `NOT col NOT LIKE` double negation; simplify |
| `Convention/NoCharType` | Avoid `CHAR` type; use `VARCHAR` or `TEXT` |
| `Convention/NoCharindexFunction` | `CHARINDEX` is SQL Server–specific; use `POSITION` or `STRPOS` |
| `Convention/NoCurrentTimestampInWhere` | `CURRENT_TIMESTAMP` in `WHERE` prevents index use |
| `Convention/NoDecodeFunction` | `DECODE` is Oracle-specific; use `CASE WHEN` |
| `Convention/NoDualTable` | `SELECT ... FROM DUAL` is Oracle-specific |
| `Convention/NoIFFunction` | `IF()` is MySQL-specific; use `CASE WHEN` |
| `Convention/NoIsnullFunction` | `ISNULL()` is SQL Server–specific; use `IS NULL` or `COALESCE` |
| `Convention/NoMinusOperator` | `MINUS` is Oracle-specific; use `EXCEPT` |
| `Convention/NoNullDefault` | Column default value is NULL (implicit — make it explicit or choose a real default) |
| `Convention/NoNvl2` | `NVL2` is Oracle-specific; use `CASE WHEN` |
| `Convention/NoRownum` | `ROWNUM` is Oracle-specific; use `ROW_NUMBER()` |
| `Convention/NoSelectAll` | `SELECT ALL` is redundant (equivalent to omitting `ALL`) |
| `Convention/NoSysdate` | `SYSDATE` is Oracle-specific; use `CURRENT_TIMESTAMP` |
| `Convention/NoUsingClause` | `JOIN ... USING` can hide ambiguity; prefer `ON` |
| `Convention/NoValuesFunction` | `VALUES()` outside INSERT context is dialect-specific |
| `Convention/NotEqual` | Use `<>` not `!=` for portability |
| `Convention/NullableConcat` | Concatenating a nullable column without null-guarding |
| `Convention/NvlFunction` | `NVL` is Oracle-specific; use `COALESCE` |
| `Convention/OrInsteadOfIn` | `col = a OR col = b` should be `col IN (a, b)` |
| `Convention/OrderByWithOffset` | `ORDER BY` with `OFFSET` without `FETCH`/`LIMIT` |
| `Convention/PivotUnpivot` | `PIVOT`/`UNPIVOT` are SQL Server–specific |
| `Convention/PreferExtract` | Use `EXTRACT(YEAR FROM col)` over `YEAR(col)` |
| `Convention/RedundantAlias` | Alias that is identical to the column name |
| `Convention/SelectDistinctStar` | `SELECT DISTINCT *` — distinct on all columns |
| `Convention/SelectStar` | Avoid `SELECT *`; list columns explicitly |
| `Convention/SelectTopN` | `SELECT TOP N` is SQL Server–specific; use `LIMIT` |
| `Convention/StringAggSeparator` | `STRING_AGG` without a separator argument |
| `Convention/TopNWithoutOrder` | `TOP N` / `LIMIT N` without `ORDER BY` returns arbitrary rows |
| `Convention/TrailingComma` | Trailing comma after the last item in a list |
| `Convention/TryCast` | `TRY_CAST` is SQL Server–specific |
| `Convention/UnnecessaryCaseWhen` | `CASE WHEN x THEN TRUE ELSE FALSE END` — simplify to `x` |
| `Convention/UpperLower` | Use `UPPER`/`LOWER` consistently (not `UCASE`/`LCASE`) |
| `Convention/UseCurrentDate` | Use `CURRENT_DATE` not `CAST(CURRENT_TIMESTAMP AS DATE)` |

---

## Layout (66 rules)

Rules that enforce formatting and whitespace conventions.

| Rule | Description |
|------|-------------|
| `Layout/AliasOnNewLine` | Alias (`AS`) should be on the same line as the expression |
| `Layout/ArithmeticOperatorAtLineEnd` | Arithmetic operators should be at the end of the line, not the start |
| `Layout/ArithmeticOperatorPadding` | Spaces required around arithmetic operators |
| `Layout/BlankLineAfterCte` | Missing blank line after the CTE block |
| `Layout/BlankLineBetweenCTEs` | Missing blank lines between CTE definitions |
| `Layout/BlankLineBetweenStatements` | Missing blank line between top-level statements |
| `Layout/ClauseOnNewLine` | SQL clauses (`SELECT`, `FROM`, `WHERE`, etc.) should be on new lines |
| `Layout/ClosingParenNewLine` | Closing parenthesis should be on its own line |
| `Layout/CommaAfterLastColumn` | Trailing comma after the last column in a SELECT list |
| `Layout/CommentSpacing` | Comments should have a space after `--` or `/*` |
| `Layout/CommentStyle` | Inconsistent comment style (`--` vs `/* */`) |
| `Layout/ComparisonOperatorSpacing` | Spaces required around comparison operators |
| `Layout/ConsistentCommentStyle` | Mix of inline and block comments |
| `Layout/ConsistentQuoteStyle` | Inconsistent use of single vs double quotes |
| `Layout/FunctionCallSpacing` | No space allowed between function name and opening parenthesis |
| `Layout/GroupByOnNewLine` | `GROUP BY` clause should start on a new line |
| `Layout/HavingOnNewLine` | `HAVING` clause should start on a new line |
| `Layout/IndentationConsistency` | Inconsistent indentation (mixed tabs and spaces, or varying levels) |
| `Layout/JoinOnNewLine` | `JOIN` clauses should start on new lines |
| `Layout/LeadingComma` | Comma at the start of a line instead of the end |
| `Layout/LeadingOperator` | Binary operator at the start of a line instead of the end |
| `Layout/LimitOnNewLine` | `LIMIT` clause should start on a new line |
| `Layout/LongLines` | Lines exceeding the configured maximum length |
| `Layout/MaxBlankLines` | Consecutive blank lines exceeding the configured maximum |
| `Layout/MaxIdentifierLength` | Identifier (table/column name) exceeds max length |
| `Layout/MaxLineCount` | File exceeds the configured maximum line count |
| `Layout/MaxStatementLength` | Single SQL statement exceeds the configured max line count |
| `Layout/MixedLineEndings` | File mixes `\r\n` (Windows) and `\n` (Unix) line endings |
| `Layout/NestedParentheses` | Unnecessary nested parentheses |
| `Layout/NoDoubleSpaces` | Multiple consecutive spaces outside string literals |
| `Layout/NoMultipleStatementsOnLine` | Multiple SQL statements on a single line |
| `Layout/NoSpaceAfterUnaryMinus` | Space between unary minus and its operand |
| `Layout/NoSpaceAroundDot` | Spaces around the `.` in qualified identifiers |
| `Layout/NoSpaceBeforeOpenParen` | Space before opening parenthesis in a function call |
| `Layout/NoSpaceInsideBrackets` | Spaces inside `[]` brackets |
| `Layout/OperatorAtLineStart` | Binary operator at start of line instead of end |
| `Layout/OrderByOnNewLine` | `ORDER BY` clause should start on a new line |
| `Layout/ParenthesisSpacing` | Spaces inside parentheses |
| `Layout/SelectColumnPerLine` | Each selected column should be on its own line |
| `Layout/SelectStarSpacing` | Spacing around `SELECT *` |
| `Layout/SelectTargetNewLine` | SELECT targets should start on a new line after `SELECT` |
| `Layout/SetOperatorNewLine` | `UNION`/`INTERSECT`/`EXCEPT` should be on their own line |
| `Layout/SingleSpaceAfterComma` | Commas should be followed by exactly one space |
| `Layout/SpaceAfterAs` | Space required after `AS` keyword |
| `Layout/SpaceAfterKeyword` | Space required after SQL keywords |
| `Layout/SpaceAfterNot` | Space required after `NOT` |
| `Layout/SpaceAfterSemicolon` | Space or newline required after `;` |
| `Layout/SpaceAroundConcatOperator` | Spaces required around `||` concat operator |
| `Layout/SpaceAroundEquals` | Spaces required around `=` in SET clauses |
| `Layout/SpaceBeforeComma` | Space before a comma |
| `Layout/SpaceBeforeIn` | Space required before `IN` keyword |
| `Layout/StatementSemicolons` | SQL statements should end with `;` |
| `Layout/TabIndentation` | Tab characters used for indentation instead of spaces |
| `Layout/TrailingBlankLines` | Trailing blank lines at the end of the file |
| `Layout/TrailingNewline` | File should end with a single newline |
| `Layout/TrailingWhitespace` | Trailing whitespace on a line |
| `Layout/UnicodeIdentifiers` | Non-ASCII characters in identifiers |
| `Layout/UnnecessaryAliasQuoting` | Alias is quoted unnecessarily |
| `Layout/WhereOnNewLine` | `WHERE` clause should start on a new line |
| `Layout/WhitespaceBeforeSemicolon` | Whitespace before the closing `;` |

---

## Lint (63 rules)

Rules that detect correctness issues, anti-patterns, and dangerous DDL.

| Rule | Description |
|------|-------------|
| `Lint/AddColumnWithoutDefault` | `ALTER TABLE ... ADD COLUMN` without a default on a non-NULL column |
| `Lint/AlterColumnType` | `ALTER COLUMN` type change may truncate existing data |
| `Lint/AlterTableAddNotNullWithoutDefault` | Adding a NOT NULL column without a DEFAULT on a populated table |
| `Lint/AlterTableDropColumn` | `DROP COLUMN` is irreversible |
| `Lint/AlterTableRenameColumn` | `RENAME COLUMN` may break dependent views or application code |
| `Lint/AlterTableSetNotNull` | Setting NOT NULL constraint on an existing column may fail |
| `Lint/ColumnAliasInWhere` | Column alias defined in SELECT used in WHERE (non-standard) |
| `Lint/CommentWithoutSpace` | Inline comment immediately after `--` without a space |
| `Lint/ConsecutiveSemicolons` | Multiple consecutive `;` statement terminators |
| `Lint/CreateIndexIfNotExists` | `CREATE INDEX IF NOT EXISTS` is not supported in all dialects |
| `Lint/CreateOrReplace` | `CREATE OR REPLACE` silently drops and recreates the object |
| `Lint/CreateSchemaStatement` | Creating a schema; flag for review |
| `Lint/CreateSequenceStatement` | Creating a sequence; flag for review |
| `Lint/CreateTableWithoutPrimaryKey` | `CREATE TABLE` without a `PRIMARY KEY` |
| `Lint/CreateTempTable` | Creating a temporary table; flag for review |
| `Lint/CreateViewWithSelectStar` | View defined with `SELECT *` will not reflect schema changes |
| `Lint/CrossDatabaseReference` | Cross-database reference (three-part name) |
| `Lint/DeleteWithoutWhere` | `DELETE` without a `WHERE` clause deletes all rows |
| `Lint/DropColumnIfExists` | `DROP COLUMN IF EXISTS` is not supported in all dialects |
| `Lint/DropIndex` | `DROP INDEX` — flag for review |
| `Lint/DropSchemaStatement` | `DROP SCHEMA` — flag for review |
| `Lint/DropTableIfExists` | `DROP TABLE IF EXISTS` — flag for review |
| `Lint/DropViewIfExists` | `DROP VIEW IF EXISTS` — flag for review |
| `Lint/DuplicateAlias` | Two tables or columns with the same alias in the same scope |
| `Lint/DuplicateColumnInCreate` | Duplicate column name in `CREATE TABLE` |
| `Lint/DuplicateCondition` | Same condition appears twice in a `WHERE` or `HAVING` clause |
| `Lint/DuplicateCteNames` | Two CTEs with the same name in the same `WITH` clause |
| `Lint/DuplicateJoin` | Same table joined more than once with identical conditions |
| `Lint/DuplicateSelectColumn` | Same column selected more than once |
| `Lint/EmptyInList` | `IN ()` with an empty list always evaluates to FALSE |
| `Lint/EmptyStringComparison` | `col = ''` — prefer `col IS NULL OR col = ''` or `TRIM(col) = ''` |
| `Lint/ExecuteStatement` | `EXECUTE`/`EXEC` dynamic SQL statement |
| `Lint/GrantAllPrivileges` | `GRANT ALL PRIVILEGES` — overly permissive |
| `Lint/InsertIgnore` | `INSERT IGNORE` is MySQL-specific and silently swallows errors |
| `Lint/InsertOrReplace` | `INSERT OR REPLACE` is SQLite-specific |
| `Lint/InsertOverwrite` | `INSERT OVERWRITE` is BigQuery/Hive-specific |
| `Lint/InsertWithoutColumnList` | `INSERT INTO table VALUES(...)` without a column list |
| `Lint/KeywordIdentifier` | Table or column name is a reserved SQL keyword |
| `Lint/MergeStatement` | `MERGE` statement — complex semantics, flag for review |
| `Lint/MultiplePrimaryKeys` | `CREATE TABLE` with multiple `PRIMARY KEY` constraints |
| `Lint/NegatedIsNull` | `NOT col IS NULL` — use `col IS NOT NULL` |
| `Lint/NonDeterministicFunction` | `RAND()`, `NEWID()`, `UUID()` are non-deterministic |
| `Lint/NullInNotIn` | `NULL` in a `NOT IN` list causes the whole expression to be UNKNOWN |
| `Lint/OnConflictClause` | `ON CONFLICT` is PostgreSQL/SQLite-specific |
| `Lint/OrderByInView` | `ORDER BY` in a view definition is non-standard |
| `Lint/RecursiveCte` | Recursive CTE — flag for complexity review |
| `Lint/SelectForUpdate` | `SELECT ... FOR UPDATE` acquires row locks |
| `Lint/SelectIntoTable` | `SELECT INTO` creates a new table (SQL Server syntax) |
| `Lint/SelectWithoutFrom` | `SELECT` without a `FROM` clause (Oracle/MySQL extension) |
| `Lint/SetVariableStatement` | `SET @var` / `SET variable` — flag for review |
| `Lint/SubqueryWithoutAlias` | Subquery in `FROM` without an alias |
| `Lint/TruncateTable` | `TRUNCATE TABLE` is irreversible |
| `Lint/UnusedCte` | CTE defined but never referenced |
| `Lint/UnusedTableAlias` | Table alias defined but never used |
| `Lint/UpdateSetDuplicate` | Same column updated twice in a single `UPDATE SET` |
| `Lint/UpdateWithoutWhere` | `UPDATE` without a `WHERE` clause updates all rows |
| `Lint/WhereTautology` | `WHERE 1=1` or always-true condition |

---

## Structure (63 rules)

Rules that flag complex or potentially inefficient query structures.

| Rule | Description |
|------|-------------|
| `Structure/AggregateInWhere` | Aggregate function in `WHERE` clause; use `HAVING` |
| `Structure/AggregateStar` | `AGG(*)` other than `COUNT(*)` |
| `Structure/AntiJoinPattern` | `LEFT JOIN ... WHERE right.id IS NULL` anti-join pattern |
| `Structure/CaseWhenCount` | `CASE WHEN` with many branches (complex conditional) |
| `Structure/ColumnCount` | SELECT list exceeds configured max column count |
| `Structure/CorrelatedSubquery` | Correlated subquery re-executes for every outer row |
| `Structure/CountDistinctInGroup` | `COUNT(DISTINCT x)` inside a `GROUP BY` query |
| `Structure/CrossApply` | `CROSS APPLY` is SQL Server–specific |
| `Structure/DeepCteChain` | CTE chain exceeds configured nesting depth |
| `Structure/DistinctGroupBy` | `DISTINCT` combined with `GROUP BY` |
| `Structure/ExceptAll` | `EXCEPT ALL` is not supported in all dialects |
| `Structure/ExcessiveGroupByColumns` | `GROUP BY` with many columns |
| `Structure/ExcessiveUnionChain` | Long `UNION` chain (consider a different approach) |
| `Structure/ExcessiveWhereConditions` | `WHERE` clause with many conditions |
| `Structure/FunctionCallDepth` | Deeply nested function calls |
| `Structure/HavingConditionsCount` | `HAVING` clause with many conditions |
| `Structure/HavingWithoutAggregate` | `HAVING` clause without any aggregate function |
| `Structure/HavingWithoutSelectAgg` | `HAVING` aggregates not reflected in SELECT list |
| `Structure/InsertSelectStar` | `INSERT INTO ... SELECT *` |
| `Structure/InsertValuesLimit` | `INSERT ... VALUES` with many rows (consider bulk load) |
| `Structure/LargeOffset` | `OFFSET` value is very large (full-scan performance risk) |
| `Structure/LateralColumnAlias` | Referencing a SELECT alias in the same SELECT list |
| `Structure/LateralJoin` | `LATERAL JOIN` is not supported in all dialects |
| `Structure/LimitWithoutOrderBy` | `LIMIT`/`TOP` without `ORDER BY` returns arbitrary rows |
| `Structure/MaxJoinOnConditions` | `JOIN ON` clause with many conditions |
| `Structure/MaxSelectColumns` | SELECT list exceeds configured max column count |
| `Structure/MixedAggregateAndColumns` | Non-aggregated columns mixed with aggregate functions without `GROUP BY` |
| `Structure/NaturalJoin` | `NATURAL JOIN` — join columns are implicit |
| `Structure/NestedAggregate` | Aggregate function nested inside another aggregate |
| `Structure/NestedCaseInElse` | `CASE` nested in the `ELSE` branch of another `CASE` |
| `Structure/NestedSubquery` | Subquery nested inside another subquery |
| `Structure/OrderByInSubquery` | `ORDER BY` in a subquery (not in the outermost query) |
| `Structure/ScalarSubqueryInSelect` | Scalar subquery in SELECT list |
| `Structure/SelectOnlyLiterals` | `SELECT` only literal values (no columns) |
| `Structure/SelectStarInCTE` | `SELECT *` inside a CTE definition |
| `Structure/SetOpPrecedence` | `UNION`/`INTERSECT`/`EXCEPT` without explicit parentheses |
| `Structure/SubqueryInHaving` | Subquery in `HAVING` clause |
| `Structure/SubqueryInJoinCondition` | Subquery in a `JOIN ON` condition |
| `Structure/SubqueryInSelect` | Subquery in SELECT list (use JOIN instead) |
| `Structure/TooManyCtes` | `WITH` clause exceeds configured max CTE count |
| `Structure/TooManyJoins` | Query has more joins than the configured maximum |
| `Structure/TooManyOrderByColumns` | `ORDER BY` list exceeds configured max columns |
| `Structure/TooManySubqueries` | Query exceeds the configured max subquery depth/count |
| `Structure/TooManyUnions` | `UNION` chain exceeds configured maximum |
| `Structure/TooManyWindowFunctions` | Too many window functions in a single query |
| `Structure/UnionAll` | `UNION` without `ALL` performs deduplication (may be unintentional) |
| `Structure/UnionBranchLimit` | `UNION` has more branches than the configured maximum |
| `Structure/UnqualifiedColumnInJoin` | Column reference not qualified with a table name in a multi-table query |
| `Structure/UnusedJoin` | Joined table not referenced in SELECT, WHERE, or HAVING |
| `Structure/UpdateWithJoin` | `UPDATE ... JOIN` syntax is MySQL-specific |
| `Structure/WildcardInUnion` | `SELECT *` in a `UNION` branch |
| `Structure/WindowFrameAllRows` | `ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING` — scans all rows |
| `Structure/WindowFrameFullPartition` | Window frame covers the full partition |
| `Structure/WindowFunctionInWhere` | Window function in `WHERE` clause (not allowed without subquery) |
| `Structure/WindowWithoutOrderBy` | Window function without `ORDER BY` in the frame |
| `Structure/ZeroLimitClause` | `LIMIT 0` returns no rows |

---

## Configuring rules

**Disable a rule:**

```toml
[rules]
disable = ["Convention/SelectStar"]
```

**Disable from CLI:**

```bash
sqrust rules --disable Convention/SelectStar
```

**List all rules with their enabled/disabled status:**

```bash
sqrust rules
sqrust rules --category Layout
sqrust rules --category Convention
```
