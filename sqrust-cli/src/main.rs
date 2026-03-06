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
use std::path::PathBuf;
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

fn main() {
    let cli = Cli::parse();
    let rules = rules();

    match cli.command {
        Commands::Check { paths } => {
            let files = collect_sql_files(&paths);
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
                    rules
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

        Commands::Fmt { paths } => {
            let files = collect_sql_files(&paths);
            for path in &files {
                let source = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error reading {}: {}", path.display(), e);
                        continue;
                    }
                };
                let ctx = FileContext::from_source(&source, &path.to_string_lossy());
                for rule in &rules {
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
