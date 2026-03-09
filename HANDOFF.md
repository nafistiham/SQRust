# SQRust — Machine Handoff Doc

Generated: 2026-03-09

## Quick Summary

SQRust is a SQL linter + formatter in Rust — think "Ruff for SQL". It competes with sqlfluff (~80 rules, Python, slow) and sqruff (~30 rules, Rust, fast). We have 155 rules across 16 waves with all tests green.

---

## Setup on New Machine

### 1. Install Prerequisites

```bash
# Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify
rustc --version   # should be stable 1.7x+
cargo --version
```

### 2. Clone the Repo

```bash
mkdir -p ~/Desktop/Learn/Projects/Personal
cd ~/Desktop/Learn/Projects/Personal
git clone https://github.com/nafistiham/SQRust.git SQRust
cd SQRust
git checkout develop
```

### 3. Create `.claude/settings.local.json`

Already in the repo (tracked). Just verify it's there:

```bash
cat .claude/settings.local.json   # should show Bash(*) + WebFetch permissions
```

If missing, create it:

```json
{
  "permissions": {
    "allow": [
      "Bash(*)",
      "WebSearch",
      "WebFetch(domain:github.com)",
      "WebFetch(domain:crates.io)",
      "WebFetch(domain:doc.rust-lang.org)",
      "WebFetch(domain:docs.sqlfluff.com)",
      "WebFetch(domain:postgresql.org)",
      "WebFetch(domain:duckdb.org)"
    ]
  }
}
```

### 4. Global Claude Setup

Create `~/.claude/CLAUDE.md`:

```markdown
# Global Claude Instructions

## Commits
- Never add `Co-Authored-By` or any co-authorship trailer to commit messages
```

### 5. Build & Test

```bash
~/.cargo/bin/cargo build --workspace
~/.cargo/bin/cargo test --workspace
# All tests should pass — 0 failures
```

---

## Project State

### Architecture

```
SQRust/
├── sqrust-core/          # Rule trait, FileContext, Diagnostic types
├── sqrust-rules/
│   ├── src/
│   │   ├── ambiguous/    # 20+ rules
│   │   ├── capitalisation/  # 4 rules
│   │   ├── convention/   # 25+ rules
│   │   ├── layout/       # 25+ rules
│   │   ├── lint/         # 25+ rules
│   │   └── structure/    # 20+ rules
│   └── tests/            # One test file per rule, ~14 tests each
└── sqrust-cli/           # sqrust check / sqrust fmt binary
```

### Rule Count: 155 (Waves 1–16 complete)

| Wave | Rules Added | Categories |
|------|-------------|------------|
| 1–7  | 50          | Core rules across all categories |
| 8    | 10          | NoCharType, NoUsingClause, CaseWhenCount, etc. |
| 9    | 10          | InSingleValue, SelectDistinctStar, LargeInList, etc. |
| 10   | 10          | LikeWithoutWildcard, NaturalJoin, LeadingComma, etc. |
| 11   | 10          | ColonCast, NestedParentheses, CommentStyle, etc. |
| 12   | 10          | ExistsOverIn, NullsOrdering, ArithmeticOperatorAtLineEnd, etc. |
| 13   | 10          | LikeTautology, RecursiveCte, ChainedComparisons, etc. |
| 14   | 10          | SelectTopN, EmptyInList, SelfJoin, MaxIdentifierLength, etc. |
| 15   | 10          | NoNullDefault, AggregateInWhere, SubqueryInOrderBy, etc. |
| 16   | 10          | LeftJoin, JoinConditionStyle, UnusedTableAlias, ConsecutiveSemicolons, NestedCaseInElse, UnusedJoin, InconsistentOrderByDirection, InconsistentColumnReference, SelectTargetNewLine, SetOperatorNewLine |

### Branch Strategy

- `develop` — active work branch
- `main` — always mirrors develop (ff-only merges)
- `production` — not used yet

All 3 branches are in sync. Always work on `develop`.

### Commit Pattern

5 atomic commits per wave (one per category: convention, lint, structure, ambiguous, layout) + 1 CLI commit. All pushed immediately.

---

## What's Done vs What's Missing

### Done ✅
- Full Cargo workspace + Rule trait
- 145 lint rules with tests
- `sqrust check` (parallel via rayon)
- `sqrust fmt` (auto-fix for layout rules)
- CI-ready (all tests green)

### Not Done ❌ (v0.1.0 gaps)
1. **`sqrust.toml` config** — no way to disable/configure rules yet
2. **Severity levels** — all rules have same weight; no ERROR/WARNING/HINT
3. **Dialect support** — ANSI only via sqlparser-rs; BigQuery/Snowflake syntax not handled
4. **README.md** — no public documentation yet
5. **Benchmarks** — no perf comparison vs sqlfluff/sqruff
6. **GitHub Actions CI** — no automated testing on push

---

## Next Steps (Priority Order)

### Option A — Keep Adding Rules (Wave 17)
Run 5 parallel agents, each implementing 2 new rules. Use the established pattern:
- Agent per category (convention / lint / structure / ambiguous / layout)
- TDD: write tests first, then implement
- ≥13 tests per rule
- Atomic commit per category

### Option B — Build sqrust.toml Config
Allow users to enable/disable rules and set thresholds in `sqrust.toml`. Key design:
```toml
[rules]
disabled_by_default = false

[rules."Layout/LongLines"]
enabled = true
max_length = 120

[rules."Structure/TooManyJoins"]
enabled = false
```

### Option C — README + Benchmarks
Write the public face of the project and run performance comparisons.

---

## How to Continue Wave 17

Dispatch 5 parallel agents in Claude Code (one message, 5 Task tool calls):

```
Agent A — Convention: <Rule1> + <Rule2>
Agent B — Lint:       <Rule3> + <Rule4>
Agent C — Structure:  <Rule5> + <Rule6>
Agent D — Ambiguous:  <Rule7> + <Rule8>
Agent E — Layout:     <Rule9> + <Rule10>
```

Each agent prompt must specify:
- Project location + cargo binary path
- Scope: their category's `mod.rs` only
- TDD: test file first (RED), then implementation (GREEN)
- ≥13 tests per rule
- `~/.cargo/bin/cargo test` to verify

After all 5 complete: run full suite, fix failures, make 5 commits, update CLI, push.

---

## Key File Paths

| Path | Purpose |
|------|---------|
| `sqrust-rules/src/convention/mod.rs` | Register new convention rules |
| `sqrust-rules/src/lint/mod.rs` | Register new lint rules |
| `sqrust-rules/src/structure/mod.rs` | Register new structure rules |
| `sqrust-rules/src/ambiguous/mod.rs` | Register new ambiguous rules |
| `sqrust-rules/src/layout/mod.rs` | Register new layout rules |
| `sqrust-cli/src/main.rs` | Register rules in CLI binary |
| `sqrust-core/src/lib.rs` | Rule trait + FileContext + Diagnostic |

## Cargo Binary

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | grep -E "FAILED|^test result"
~/.cargo/bin/cargo build -p sqrust-cli
```

## Note on `internals/`

The `internals/` folder (competition research, decisions, plans) is **gitignored** and will NOT be on the new machine. It contains private notes and is not needed to continue development — all context is captured in this doc and in `.claude/` memory.
