use clap::{Parser, Subcommand};
use rayon::prelude::*;
use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::trailing_whitespace::TrailingWhitespace;
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
    vec![Box::new(TrailingWhitespace)]
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
