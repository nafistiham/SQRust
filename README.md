# SQRust

[![CI](https://github.com/nafistiham/SQRust/actions/workflows/ci.yml/badge.svg)](https://github.com/nafistiham/SQRust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**A fast SQL linter written in Rust.** The Ruff for SQL.

165 rules. Single binary. No Python required.

---

## Why SQRust?

If you use sqlfluff, you know the pain: linting a 200-file dbt project takes minutes in CI. SQRust solves that.

Benchmarked on **500 real SQL files** from [GitLab's public dbt project](https://gitlab.com/gitlab-data/analytics) (32,000 lines of production Snowflake SQL):

| Tool | Time | Rules |
|------|------|-------|
| **SQRust** | **57 ms** | **165** |
| sqruff 0.34.1 | 588 ms | ~30 |
| sqlfluff 4.0.4 | 38,409 ms | ~80 |

> **10× faster than sqruff. 682× faster than sqlfluff. More rules than both combined.**

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) (5 runs, real corpus, all tools in ANSI mode).
[Run the benchmark yourself](#run-the-benchmark-yourself).

---

## Install

**Pre-built binary (recommended):**

```bash
# macOS (Apple Silicon)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/

# macOS (Intel)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-x86_64-apple-darwin.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/

# Linux (x86_64)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/
```

**Build from source:**

```bash
git clone https://github.com/nafistiham/SQRust.git
cd SQRust
cargo build -p sqrust-cli --release
# binary at: target/release/sqrust
```

---

## Usage

```bash
# Lint a directory
sqrust check models/

# Lint a single file
sqrust check query.sql

# Auto-fix layout issues
sqrust fmt models/
```

**Output:**

```
models/orders.sql:12:5: [Convention/SelectStar] Avoid SELECT *; list columns explicitly
models/orders.sql:34:1: [Layout/TrailingWhitespace] Trailing whitespace on line
models/payments.sql:8:24: [Convention/ColonCast] Avoid PostgreSQL-style ::cast; use CAST() instead
models/payments.sql:41:1: [Layout/LongLines] Line exceeds 120 characters (was 143)
```

---

## Rules

165 rules across 5 categories, mapped to sqlfluff's catalog where applicable.

| Category | Rules | Examples |
|----------|-------|---------|
| **Convention** | 35 | `SelectStar`, `ColonCast`, `NotEqual`, `IsNull`, `TrailingComma` |
| **Layout** | 33 | `LongLines`, `TrailingWhitespace`, `ClauseOnNewLine`, `LeadingComma` |
| **Lint** | 27 | `UnreferencedCTE`, `ColumnAliasInWhere`, `DuplicateJoin` |
| **Structure** | 22 | `WildcardInUnion`, `NaturalJoin`, `UnqualifiedColumnInJoin` |
| **Ambiguous** | 31 | `FloatingPointComparison`, `AmbiguousDateFormat`, `ImplicitCrossJoin` |

Full rule list → [docs/rules.md](docs/rules.md) _(coming soon)_

---

## Comparison

|  | SQRust | sqruff | sqlfluff |
|--|--------|--------|----------|
| Language | Rust | Rust | Python |
| Rules | **165** | ~30 | ~80 |
| Speed (500 files) | **62 ms** | 569 ms | 38,000 ms |
| Single binary | ✅ | ✅ | ❌ |
| Auto-fix | ✅ | ✅ | ✅ |
| Config file | 🚧 | ✅ | ✅ |
| Dialect support | ANSI | ANSI+ | Many |
| dbt-ready | ✅ | ✅ | ✅ |

> Config file (`sqrust.toml`) is in active development.

---

## Pre-commit

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/nafistiham/SQRust
    rev: v0.1.0
    hooks:
      - id: sqrust
        args: [check]
```

---

## Run the Benchmark Yourself

```bash
curl -sSL https://raw.githubusercontent.com/nafistiham/SQRust/main/bench/benchmark.sh | bash
```

Or clone and run:

```bash
git clone https://github.com/nafistiham/SQRust
cd SQRust/bench
./benchmark.sh            # uses jaffle-shop as corpus
./benchmark.sh /your/dbt/project   # or point at your own project
```

Requires: `hyperfine`, `sqlfluff`, `sqruff`, `sqrust` on PATH.

---

## Contributing

```bash
git clone https://github.com/nafistiham/SQRust
cd SQRust
cargo test --workspace   # all tests must pass
```

Each rule lives in `sqrust-rules/src/<category>/<rule_name>.rs` with a test file in `sqrust-rules/tests/`.
Every rule has ≥13 tests. We use TDD — tests first.

---

## License

MIT
