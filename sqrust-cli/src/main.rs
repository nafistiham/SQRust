use clap::{Parser, Subcommand};
use rayon::prelude::*;
use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::column_name_conflict::ColumnNameConflict;
use sqrust_rules::ambiguous::group_by_position::GroupByPosition;
use sqrust_rules::ambiguous::redundant_between::RedundantBetween;
use sqrust_rules::ambiguous::self_comparison::SelfComparison;
use sqrust_rules::ambiguous::having_without_group_by::HavingWithoutGroupBy;
use sqrust_rules::ambiguous::implicit_cross_join::ImplicitCrossJoin;
use sqrust_rules::ambiguous::join_without_condition::JoinWithoutCondition;
use sqrust_rules::ambiguous::order_by_position::OrderByPosition;
use sqrust_rules::ambiguous::select_star_with_other_columns::SelectStarWithOtherColumns;
use sqrust_rules::ambiguous::table_alias_conflict::TableAliasConflict;
use sqrust_rules::ambiguous::unaliased_expression::UnaliasedExpression;
use sqrust_rules::ambiguous::union_column_mismatch::UnionColumnMismatch;
use sqrust_rules::capitalisation::functions::Functions;
use sqrust_rules::capitalisation::keywords::Keywords;
use sqrust_rules::capitalisation::literals::Literals;
use sqrust_rules::capitalisation::types::Types;
use sqrust_rules::convention::boolean_comparison::BooleanComparison;
use sqrust_rules::convention::case_else::CaseElse;
use sqrust_rules::convention::coalesce::Coalesce;
use sqrust_rules::convention::like_percent_only::LikePercentOnly;
use sqrust_rules::convention::no_select_all::NoSelectAll;
use sqrust_rules::convention::unnecessary_else_null::UnnecessaryElseNull;
use sqrust_rules::convention::comma_style::CommaStyle;
use sqrust_rules::convention::count_star::CountStar;
use sqrust_rules::convention::distinct_parenthesis::DistinctParenthesis;
use sqrust_rules::convention::in_null_comparison::InNullComparison;
use sqrust_rules::convention::is_null::IsNull;
use sqrust_rules::convention::not_equal::NotEqual;
use sqrust_rules::convention::order_by_with_offset::OrderByWithOffset;
use sqrust_rules::convention::select_star::SelectStar;
use sqrust_rules::convention::trailing_comma::TrailingComma;
use sqrust_rules::layout::comment_spacing::CommentSpacing;
use sqrust_rules::layout::long_lines::LongLines;
use sqrust_rules::layout::max_blank_lines::MaxBlankLines;
use sqrust_rules::layout::parenthesis_spacing::ParenthesisSpacing;
use sqrust_rules::layout::no_double_spaces::NoDoubleSpaces;
use sqrust_rules::layout::statement_semicolons::StatementSemicolons;
use sqrust_rules::layout::single_space_after_comma::SingleSpaceAfterComma;
use sqrust_rules::layout::space_around_equals::SpaceAroundEquals;
use sqrust_rules::layout::space_before_comma::SpaceBeforeComma;
use sqrust_rules::layout::tab_indentation::TabIndentation;
use sqrust_rules::layout::trailing_blank_lines::TrailingBlankLines;
use sqrust_rules::layout::trailing_newline::TrailingNewline;
use sqrust_rules::layout::trailing_whitespace::TrailingWhitespace;
use sqrust_rules::lint::delete_without_where::DeleteWithoutWhere;
use sqrust_rules::lint::duplicate_alias::DuplicateAlias;
use sqrust_rules::lint::empty_string_comparison::EmptyStringComparison;
use sqrust_rules::lint::insert_without_column_list::InsertWithoutColumnList;
use sqrust_rules::lint::update_set_duplicate::UpdateSetDuplicate;
use sqrust_rules::lint::where_tautology::WhereTautology;
use sqrust_rules::lint::duplicate_cte_names::DuplicateCteNames;
use sqrust_rules::lint::negated_is_null::NegatedIsNull;
use sqrust_rules::lint::unused_cte::UnusedCte;
use sqrust_rules::lint::update_without_where::UpdateWithoutWhere;
use sqrust_rules::structure::column_count::ColumnCount;
use sqrust_rules::structure::distinct_group_by::DistinctGroupBy;
use sqrust_rules::structure::too_many_joins::TooManyJoins;
use sqrust_rules::structure::window_without_order_by::WindowWithoutOrderBy;
use sqrust_rules::structure::having_without_aggregate::HavingWithoutAggregate;
use sqrust_rules::structure::too_many_ctes::TooManyCtes;
use sqrust_rules::structure::limit_without_order_by::LimitWithoutOrderBy;
use sqrust_rules::structure::nested_subquery::NestedSubquery;
use sqrust_rules::structure::subquery_in_select::SubqueryInSelect;
use sqrust_rules::structure::union_all::UnionAll;
use sqrust_rules::convention::no_char_type::NoCharType;
use sqrust_rules::convention::no_using_clause::NoUsingClause;
use sqrust_rules::lint::subquery_without_alias::SubqueryWithoutAlias;
use sqrust_rules::lint::duplicate_column_in_create::DuplicateColumnInCreate;
use sqrust_rules::structure::case_when_count::CaseWhenCount;
use sqrust_rules::structure::order_by_in_subquery::OrderByInSubquery;
use sqrust_rules::ambiguous::division_by_zero::DivisionByZero;
use sqrust_rules::ambiguous::or_in_join_condition::OrInJoinCondition;
use sqrust_rules::layout::unicode_identifiers::UnicodeIdentifiers;
use sqrust_rules::layout::mixed_line_endings::MixedLineEndings;
use sqrust_rules::convention::in_single_value::InSingleValue;
use sqrust_rules::convention::select_distinct_star::SelectDistinctStar;
use sqrust_rules::lint::null_in_not_in::NullInNotIn;
use sqrust_rules::lint::drop_table_if_exists::DropTableIfExists;
use sqrust_rules::structure::large_in_list::LargeInList;
use sqrust_rules::structure::function_call_depth::FunctionCallDepth;
use sqrust_rules::ambiguous::cross_join_keyword::CrossJoinKeyword;
use sqrust_rules::ambiguous::ambiguous_bool_op::AmbiguousBoolOp;
use sqrust_rules::layout::space_after_semicolon::SpaceAfterSemicolon;
use sqrust_rules::layout::blank_line_between_statements::BlankLineBetweenStatements;
use sqrust_rules::convention::like_without_wildcard::LikeWithoutWildcard;
use sqrust_rules::convention::concat_operator::ConcatOperator;
use sqrust_rules::lint::truncate_table::TruncateTable;
use sqrust_rules::lint::create_table_without_primary_key::CreateTableWithoutPrimaryKey;
use sqrust_rules::structure::excessive_group_by_columns::ExcessiveGroupByColumns;
use sqrust_rules::structure::natural_join::NaturalJoin;
use sqrust_rules::ambiguous::full_outer_join::FullOuterJoin;
use sqrust_rules::ambiguous::select_distinct_with_group_by::SelectDistinctWithGroupBy;
use sqrust_rules::layout::leading_comma::LeadingComma;
use sqrust_rules::layout::leading_operator::LeadingOperator;
use sqrust_rules::convention::colon_cast::ColonCast;
use sqrust_rules::convention::if_null_function::IfNullFunction;
use sqrust_rules::lint::alter_table_drop_column::AlterTableDropColumn;
use sqrust_rules::lint::create_or_replace::CreateOrReplace;
use sqrust_rules::structure::excessive_where_conditions::ExcessiveWhereConditions;
use sqrust_rules::structure::too_many_unions::TooManyUnions;
use sqrust_rules::ambiguous::window_function_without_partition::WindowFunctionWithoutPartition;
use sqrust_rules::ambiguous::select_null_expression::SelectNullExpression;
use sqrust_rules::layout::nested_parentheses::NestedParentheses;
use sqrust_rules::layout::comment_style::CommentStyle;
use sqrust_rules::convention::exists_over_in::ExistsOverIn;
use sqrust_rules::convention::no_current_timestamp_in_where::NoCurrentTimestampInWhere;
use sqrust_rules::lint::drop_schema_statement::DropSchemaStatement;
use sqrust_rules::lint::non_deterministic_function::NonDeterministicFunction;
use sqrust_rules::structure::having_conditions_count::HavingConditionsCount;
use sqrust_rules::structure::too_many_subqueries::TooManySubqueries;
use sqrust_rules::ambiguous::nulls_ordering::NullsOrdering;
use sqrust_rules::ambiguous::mixed_join_types::MixedJoinTypes;
use sqrust_rules::layout::arithmetic_operator_at_line_end::ArithmeticOperatorAtLineEnd;
use sqrust_rules::layout::max_statement_length::MaxStatementLength;
use sqrust_rules::convention::like_tautology::LikeTautology;
use sqrust_rules::convention::coalesce_null_arg::CoalesceNullArg;
use sqrust_rules::lint::recursive_cte::RecursiveCte;
use sqrust_rules::lint::insert_or_replace::InsertOrReplace;
use sqrust_rules::structure::max_join_on_conditions::MaxJoinOnConditions;
use sqrust_rules::structure::select_only_literals::SelectOnlyLiterals;
use sqrust_rules::ambiguous::chained_comparisons::ChainedComparisons;
use sqrust_rules::ambiguous::subquery_in_group_by::SubqueryInGroupBy;
use sqrust_rules::layout::consistent_comment_style::ConsistentCommentStyle;
use sqrust_rules::layout::whitespace_before_semicolon::WhitespaceBeforeSemicolon;
use sqrust_rules::convention::select_top_n::SelectTopN;
use sqrust_rules::convention::leading_zero_numeric::LeadingZeroNumeric;
use sqrust_rules::lint::empty_in_list::EmptyInList;
use sqrust_rules::lint::duplicate_condition::DuplicateCondition;
use sqrust_rules::structure::too_many_order_by_columns::TooManyOrderByColumns;
use sqrust_rules::structure::mixed_aggregate_and_columns::MixedAggregateAndColumns;
use sqrust_rules::ambiguous::self_join::SelfJoin;
use sqrust_rules::ambiguous::function_on_filtered_column::FunctionOnFilteredColumn;
use sqrust_rules::layout::max_identifier_length::MaxIdentifierLength;
use sqrust_rules::layout::clause_on_new_line::ClauseOnNewLine;
use sqrust_rules::convention::no_null_default::NoNullDefault;
use sqrust_rules::convention::unnecessary_case_when::UnnecessaryCaseWhen;
use sqrust_rules::lint::grant_all_privileges::GrantAllPrivileges;
use sqrust_rules::lint::alter_table_add_not_null_without_default::AlterTableAddNotNullWithoutDefault;
use sqrust_rules::structure::aggregate_in_where::AggregateInWhere;
use sqrust_rules::structure::zero_limit_clause::ZeroLimitClause;
use sqrust_rules::ambiguous::subquery_in_order_by::SubqueryInOrderBy;
use sqrust_rules::ambiguous::non_deterministic_group_by::NonDeterministicGroupBy;
use sqrust_rules::ambiguous::inconsistent_order_by_direction::InconsistentOrderByDirection;
use sqrust_rules::ambiguous::inconsistent_column_reference::InconsistentColumnReference;
use sqrust_rules::layout::no_multiple_statements_on_line::NoMultipleStatementsOnLine;
use sqrust_rules::layout::comparison_operator_spacing::ComparisonOperatorSpacing;
use sqrust_rules::layout::select_target_new_line::SelectTargetNewLine;
use sqrust_rules::layout::set_operator_new_line::SetOperatorNewLine;
use sqrust_rules::convention::left_join::LeftJoin;
use sqrust_rules::convention::join_condition_style::JoinConditionStyle;
// Wave 18
use sqrust_rules::convention::redundant_alias::RedundantAlias;
use sqrust_rules::convention::nullable_concat::NullableConcat;
use sqrust_rules::lint::duplicate_select_column::DuplicateSelectColumn;
use sqrust_rules::lint::keyword_identifier::KeywordIdentifier;
// Wave 17
use sqrust_rules::layout::arithmetic_operator_padding::ArithmeticOperatorPadding;
use sqrust_rules::layout::blank_line_after_cte::BlankLineAfterCte;
use sqrust_rules::ambiguous::floating_point_comparison::FloatingPointComparison;
use sqrust_rules::ambiguous::ambiguous_date_format::AmbiguousDateFormat;
use sqrust_rules::convention::explicit_alias::ExplicitAlias;
use sqrust_rules::convention::or_instead_of_in::OrInsteadOfIn;
use sqrust_rules::lint::column_alias_in_where::ColumnAliasInWhere;
use sqrust_rules::lint::duplicate_join::DuplicateJoin;
use sqrust_rules::lint::unused_table_alias::UnusedTableAlias;
use sqrust_rules::lint::consecutive_semicolons::ConsecutiveSemicolons;
use sqrust_rules::structure::nested_case_in_else::NestedCaseInElse;
use sqrust_rules::structure::unused_join::UnusedJoin;
use sqrust_rules::structure::wildcard_in_union::WildcardInUnion;
use sqrust_rules::structure::unqualified_column_in_join::UnqualifiedColumnInJoin;
// Wave 18
use sqrust_rules::layout::consistent_quote_style::ConsistentQuoteStyle;
use sqrust_rules::layout::space_around_concat_operator::SpaceAroundConcatOperator;
use sqrust_rules::structure::deep_cte_chain::DeepCteChain;
use sqrust_rules::structure::insert_select_star::InsertSelectStar;
use sqrust_rules::ambiguous::case_null_check::CaseNullCheck;
use sqrust_rules::ambiguous::multiple_count_distinct::MultipleCountDistinct;
use sqrust_rules::lint::alter_column_type::AlterColumnType;
use sqrust_rules::lint::cross_database_reference::CrossDatabaseReference;
// Wave 20
use sqrust_rules::lint::select_into_table::SelectIntoTable;
use sqrust_rules::lint::order_by_in_view::OrderByInView;
// Wave 19
use sqrust_rules::ambiguous::coalesce_with_single_arg::CoalesceWithSingleArg;
use sqrust_rules::ambiguous::in_subquery_multi_column::InSubqueryMultiColumn;
use sqrust_rules::structure::set_op_precedence::SetOpPrecedence;
use sqrust_rules::structure::window_frame_all_rows::WindowFrameAllRows;
use sqrust_rules::convention::explicit_join_type::ExplicitJoinType;
use sqrust_rules::convention::negated_not_like::NegatedNotLike;
use sqrust_rules::layout::function_call_spacing::FunctionCallSpacing;
use sqrust_rules::layout::indentation_consistency::IndentationConsistency;
// Wave 20
use sqrust_rules::layout::no_space_around_dot::NoSpaceAroundDot;
use sqrust_rules::layout::unnecessary_alias_quoting::UnnecessaryAliasQuoting;
use sqrust_rules::structure::except_all::ExceptAll;
use sqrust_rules::structure::lateral_column_alias::LateralColumnAlias;
use sqrust_rules::ambiguous::exists_select_list::ExistsSelectList;
use sqrust_rules::ambiguous::between_null_boundary::BetweenNullBoundary;
use sqrust_rules::convention::avoid_iif::AvoidIif;
use sqrust_rules::convention::cast_vs_convert::CastVsConvert;
// Wave 21
use sqrust_rules::convention::len_function::LenFunction;
use sqrust_rules::convention::upper_lower::UpperLower;
use sqrust_core::Config;
use std::path::{Path, PathBuf};
use std::process;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "sqrust", version, about = "Fast SQL linter and formatter")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check SQL files for lint violations
    Check {
        #[arg(value_name = "PATH", default_value = ".")]
        paths: Vec<PathBuf>,
    },
    /// Format SQL files (auto-fix violations)
    Fmt {
        #[arg(value_name = "PATH", default_value = ".")]
        paths: Vec<PathBuf>,
    },
}

fn rules() -> Vec<Box<dyn Rule>> {
    vec![
        // Layout
        Box::new(TrailingWhitespace),
        Box::new(TrailingNewline),
        Box::new(TrailingBlankLines),
        Box::new(TabIndentation),
        Box::new(SingleSpaceAfterComma),
        Box::new(SpaceBeforeComma),
        Box::new(LongLines::default()),
        Box::new(CommentSpacing),
        Box::new(SpaceAroundEquals),
        Box::new(NoDoubleSpaces),
        Box::new(StatementSemicolons),
        Box::new(ParenthesisSpacing),
        Box::new(MaxBlankLines::default()),
        // Capitalisation
        Box::new(Keywords),
        Box::new(Functions),
        Box::new(Types),
        Box::new(Literals),
        // Convention
        Box::new(NotEqual),
        Box::new(CommaStyle),
        Box::new(Coalesce),
        Box::new(SelectStar),
        Box::new(CountStar),
        Box::new(IsNull),
        Box::new(DistinctParenthesis),
        Box::new(TrailingComma),
        Box::new(InNullComparison),
        Box::new(CaseElse),
        Box::new(OrderByWithOffset),
        Box::new(UnnecessaryElseNull),
        Box::new(NoSelectAll),
        Box::new(BooleanComparison),
        Box::new(LikePercentOnly),
        // Ambiguous
        Box::new(GroupByPosition),
        Box::new(OrderByPosition),
        Box::new(SelectStarWithOtherColumns),
        Box::new(HavingWithoutGroupBy),
        Box::new(ImplicitCrossJoin),
        Box::new(UnaliasedExpression),
        Box::new(TableAliasConflict),
        Box::new(JoinWithoutCondition),
        Box::new(UnionColumnMismatch),
        Box::new(ColumnNameConflict),
        Box::new(SelfComparison),
        Box::new(RedundantBetween),
        // Lint
        Box::new(UnusedCte),
        Box::new(DuplicateAlias),
        Box::new(DeleteWithoutWhere),
        Box::new(UpdateWithoutWhere),
        Box::new(NegatedIsNull),
        Box::new(DuplicateCteNames),
        Box::new(WhereTautology),
        Box::new(UpdateSetDuplicate),
        Box::new(EmptyStringComparison),
        Box::new(InsertWithoutColumnList),
        // Structure
        Box::new(UnionAll),
        Box::new(LimitWithoutOrderBy),
        Box::new(NestedSubquery::default()),
        Box::new(ColumnCount::default()),
        Box::new(DistinctGroupBy),
        Box::new(SubqueryInSelect),
        Box::new(HavingWithoutAggregate),
        Box::new(TooManyJoins::default()),
        Box::new(WindowWithoutOrderBy),
        Box::new(TooManyCtes::default()),
        // Wave 8
        Box::new(NoCharType),
        Box::new(NoUsingClause),
        Box::new(SubqueryWithoutAlias),
        Box::new(DuplicateColumnInCreate),
        Box::new(CaseWhenCount::default()),
        Box::new(OrderByInSubquery),
        Box::new(DivisionByZero),
        Box::new(OrInJoinCondition),
        Box::new(UnicodeIdentifiers),
        Box::new(MixedLineEndings),
        // Wave 9
        Box::new(InSingleValue),
        Box::new(SelectDistinctStar),
        Box::new(NullInNotIn),
        Box::new(DropTableIfExists),
        Box::new(LargeInList::default()),
        Box::new(FunctionCallDepth::default()),
        Box::new(CrossJoinKeyword),
        Box::new(AmbiguousBoolOp),
        Box::new(SpaceAfterSemicolon),
        Box::new(BlankLineBetweenStatements),
        // Wave 10
        Box::new(LikeWithoutWildcard),
        Box::new(ConcatOperator),
        Box::new(TruncateTable),
        Box::new(CreateTableWithoutPrimaryKey),
        Box::new(ExcessiveGroupByColumns::default()),
        Box::new(NaturalJoin),
        Box::new(FullOuterJoin),
        Box::new(SelectDistinctWithGroupBy),
        Box::new(LeadingComma),
        Box::new(LeadingOperator),
        // Wave 11
        Box::new(ColonCast),
        Box::new(IfNullFunction),
        Box::new(AlterTableDropColumn),
        Box::new(CreateOrReplace),
        Box::new(ExcessiveWhereConditions::default()),
        Box::new(TooManyUnions::default()),
        Box::new(WindowFunctionWithoutPartition),
        Box::new(SelectNullExpression),
        Box::new(NestedParentheses::default()),
        Box::new(CommentStyle),
        // Wave 12
        Box::new(ExistsOverIn),
        Box::new(NoCurrentTimestampInWhere),
        Box::new(DropSchemaStatement),
        Box::new(NonDeterministicFunction),
        Box::new(HavingConditionsCount::default()),
        Box::new(TooManySubqueries::default()),
        Box::new(NullsOrdering),
        Box::new(MixedJoinTypes),
        Box::new(ArithmeticOperatorAtLineEnd),
        Box::new(MaxStatementLength::default()),
        // Wave 13
        Box::new(LikeTautology),
        Box::new(CoalesceNullArg),
        Box::new(RecursiveCte),
        Box::new(InsertOrReplace),
        Box::new(MaxJoinOnConditions::default()),
        Box::new(SelectOnlyLiterals),
        Box::new(ChainedComparisons),
        Box::new(SubqueryInGroupBy),
        Box::new(ConsistentCommentStyle),
        Box::new(WhitespaceBeforeSemicolon),
        // Wave 14
        Box::new(SelectTopN),
        Box::new(LeadingZeroNumeric),
        Box::new(EmptyInList),
        Box::new(DuplicateCondition),
        Box::new(TooManyOrderByColumns::default()),
        Box::new(MixedAggregateAndColumns),
        Box::new(SelfJoin),
        Box::new(FunctionOnFilteredColumn),
        Box::new(MaxIdentifierLength::default()),
        Box::new(ClauseOnNewLine),
        // Wave 15
        Box::new(NoNullDefault),
        Box::new(UnnecessaryCaseWhen),
        Box::new(GrantAllPrivileges),
        Box::new(AlterTableAddNotNullWithoutDefault),
        Box::new(AggregateInWhere),
        Box::new(ZeroLimitClause),
        Box::new(SubqueryInOrderBy),
        Box::new(NonDeterministicGroupBy),
        Box::new(NoMultipleStatementsOnLine),
        Box::new(ComparisonOperatorSpacing),
        // Wave 16
        Box::new(LeftJoin),
        Box::new(JoinConditionStyle),
        Box::new(UnusedTableAlias),
        Box::new(ConsecutiveSemicolons),
        Box::new(NestedCaseInElse),
        Box::new(UnusedJoin),
        Box::new(InconsistentOrderByDirection),
        Box::new(InconsistentColumnReference),
        Box::new(SelectTargetNewLine),
        Box::new(SetOperatorNewLine),
        // Wave 17
        Box::new(ExplicitAlias),
        Box::new(OrInsteadOfIn),
        Box::new(ColumnAliasInWhere),
        Box::new(DuplicateJoin),
        Box::new(WildcardInUnion),
        Box::new(UnqualifiedColumnInJoin),
        Box::new(FloatingPointComparison),
        Box::new(AmbiguousDateFormat),
        Box::new(ArithmeticOperatorPadding),
        Box::new(BlankLineAfterCte),
        // Wave 18
        Box::new(RedundantAlias),
        Box::new(NullableConcat),
        Box::new(ConsistentQuoteStyle),
        Box::new(SpaceAroundConcatOperator),
        Box::new(DuplicateSelectColumn),
        Box::new(KeywordIdentifier),
        Box::new(DeepCteChain::default()),
        Box::new(InsertSelectStar),
        Box::new(CaseNullCheck),
        Box::new(MultipleCountDistinct),
        // Wave 19
        Box::new(CoalesceWithSingleArg),
        Box::new(InSubqueryMultiColumn),
        Box::new(SetOpPrecedence),
        Box::new(WindowFrameAllRows),
        Box::new(ExplicitJoinType),
        Box::new(NegatedNotLike),
        // Wave 19 (layout)
        Box::new(FunctionCallSpacing),
        Box::new(IndentationConsistency),
        // Wave 19 (lint)
        Box::new(AlterColumnType),
        Box::new(CrossDatabaseReference),
        // Wave 20
        Box::new(SelectIntoTable),
        Box::new(OrderByInView),
        Box::new(NoSpaceAroundDot),
        Box::new(UnnecessaryAliasQuoting),
        Box::new(ExceptAll),
        Box::new(LateralColumnAlias),
        Box::new(ExistsSelectList),
        Box::new(BetweenNullBoundary),
        // Wave 20
        Box::new(AvoidIif),
        Box::new(CastVsConvert),
        // Wave 21
        Box::new(LenFunction),
        Box::new(AlterTableRenameColumn),
        Box::new(ConcatFunctionNullArg),
        Box::new(TooManyWindowFunctions::default()),
        Box::new(MaxLineCount::default()),
    ]
}

fn collect_sql_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
        } else {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path().to_path_buf();
                if p.extension().map_or(false, |ext| ext == "sql") {
                    files.push(p);
                }
            }
        }
    }
    files
}

/// Returns true if the path matches any of the exclude glob patterns.
/// Patterns are matched against the full path string and all path suffixes,
/// so `dbt_packages/**` matches `/project/dbt_packages/foo.sql`.
fn is_excluded(path: &Path, exclude: &[String]) -> bool {
    if exclude.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    // Collect suffix start positions (after each '/')
    let suffix_starts: Vec<usize> = path_str
        .char_indices()
        .filter(|(_, c)| *c == '/' || *c == '\\')
        .map(|(i, _)| i + 1)
        .collect();

    for pattern_str in exclude {
        let Ok(pattern) = glob::Pattern::new(pattern_str) else {
            continue;
        };
        if pattern.matches(&path_str) {
            return true;
        }
        for &start in &suffix_starts {
            if pattern.matches(&path_str[start..]) {
                return true;
            }
        }
    }
    false
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { ref paths } | Commands::Fmt { ref paths } => {
            // Load config from first path arg, or current dir.
            let config_start = paths.first().map(PathBuf::as_path).unwrap_or(Path::new("."));
            let config = match Config::load(config_start) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("sqrust: {}", e);
                    process::exit(2);
                }
            };

            let active_rules: Vec<Box<dyn Rule>> = rules()
                .into_iter()
                .filter(|r| config.rule_enabled(r.name()))
                .collect();

            let all_files = collect_sql_files(paths);
            let files: Vec<PathBuf> = all_files
                .into_iter()
                .filter(|p| !is_excluded(p, &config.sqrust.exclude))
                .collect();

            match cli.command {
                Commands::Check { .. } => {
                    if files.is_empty() {
                        eprintln!("No SQL files found.");
                        process::exit(0);
                    }

                    let violations: Vec<String> = files
                        .par_iter()
                        .flat_map(|path| {
                            let source = match std::fs::read_to_string(path) {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("Error reading {}: {}", path.display(), e);
                                    return Vec::new();
                                }
                            };
                            let ctx = FileContext::from_source(&source, &path.to_string_lossy());
                            active_rules
                                .iter()
                                .flat_map(|rule| rule.check(&ctx))
                                .map(|d| {
                                    format!(
                                        "{}:{}:{}: [{}] {}",
                                        path.display(),
                                        d.line,
                                        d.col,
                                        d.rule,
                                        d.message
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect();

                    for v in &violations {
                        println!("{}", v);
                    }

                    if !violations.is_empty() {
                        process::exit(1);
                    }
                }

                Commands::Fmt { .. } => {
                    for path in &files {
                        let source = match std::fs::read_to_string(path) {
                            Ok(s) => s,
                            Err(e) => {
                                eprintln!("Error reading {}: {}", path.display(), e);
                                continue;
                            }
                        };
                        let ctx = FileContext::from_source(&source, &path.to_string_lossy());
                        for rule in &active_rules {
                            if let Some(fixed) = rule.fix(&ctx) {
                                if fixed != source {
                                    if let Err(e) = std::fs::write(path, &fixed) {
                                        eprintln!("Error writing {}: {}", path.display(), e);
                                    } else {
                                        println!("Fixed: {}", path.display());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
