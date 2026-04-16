#!/usr/bin/env bash
# SQRust Benchmark — compare sqrust vs sqruff vs sqlfluff
#
# Usage:
#   ./benchmark.sh                    # downloads jaffle-shop as corpus
#   ./benchmark.sh /path/to/dbt       # use your own SQL directory
#
# Requirements: hyperfine, sqrust, sqruff, sqlfluff
# Install:
#   cargo install sqrust-cli sqruff
#   pip install sqlfluff
#   brew install hyperfine   (macOS) / cargo install hyperfine

set -euo pipefail

CORPUS="${1:-}"
TMPDIR_USED=0

# ── colours ────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BOLD='\033[1m'; RESET='\033[0m'

check() { command -v "$1" &>/dev/null; }
require() {
    if ! check "$1"; then
        echo -e "${RED}✗ '$1' not found.${RESET} Install: $2"
        exit 1
    fi
}

# ── check dependencies ──────────────────────────────────────────────────────
echo -e "${BOLD}Checking dependencies...${RESET}"
require hyperfine  "brew install hyperfine  (macOS) or cargo install hyperfine"
require sqrust     "cargo install sqrust-cli"
require sqruff     "cargo install sqruff"
require sqlfluff   "pip install sqlfluff"
echo -e "${GREEN}✓ All tools found${RESET}"
echo ""

# ── corpus ──────────────────────────────────────────────────────────────────
if [[ -z "$CORPUS" ]]; then
    echo -e "${BOLD}No corpus specified — downloading jaffle-shop...${RESET}"
    TMP=$(mktemp -d)
    TMPDIR_USED=1
    git clone --depth=1 --quiet https://github.com/dbt-labs/jaffle-shop.git "$TMP/jaffle-shop"
    CORPUS="$TMP/jaffle-shop"
fi

# count files and lines
SQL_FILES=$(find "$CORPUS" -name "*.sql" | wc -l | tr -d ' ')
SQL_LINES=$(find "$CORPUS" -name "*.sql" -exec cat {} \; | wc -l | tr -d ' ')

if [[ "$SQL_FILES" -eq 0 ]]; then
    echo -e "${RED}✗ No .sql files found in: $CORPUS${RESET}"
    exit 1
fi

echo -e "${BOLD}Corpus:${RESET} $CORPUS"
echo -e "  Files : $SQL_FILES"
echo -e "  Lines : $SQL_LINES"
echo ""

# ── versions ────────────────────────────────────────────────────────────────
SQRUST_VER=$(sqrust --version 2>/dev/null || echo "?")
SQRUFF_VER=$(sqruff --version 2>/dev/null || echo "?")
SQLFLUFF_VER=$(sqlfluff --version 2>/dev/null || echo "?")

echo -e "${BOLD}Versions:${RESET}"
echo "  sqrust    $SQRUST_VER"
echo "  sqruff    $SQRUFF_VER"
echo "  sqlfluff  $SQLFLUFF_VER"
echo ""

# ── warmup filesystem cache ─────────────────────────────────────────────────
find "$CORPUS" -name "*.sql" -exec cat {} \; > /dev/null

# ── benchmark ───────────────────────────────────────────────────────────────
echo -e "${BOLD}Running benchmark (this may take a few minutes for sqlfluff)...${RESET}"
echo ""

RESULTS_JSON=$(mktemp /tmp/sqrust-bench-XXXXXX.json)

hyperfine \
    --warmup 2 \
    --min-runs 5 \
    --ignore-failure \
    --export-json "$RESULTS_JSON" \
    --style basic \
    "sqrust check $CORPUS/" \
    "sqruff lint --dialect ansi $CORPUS/" \
    "sqlfluff lint --dialect ansi $CORPUS/"

# ── parse results ────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}Results:${RESET}"
echo ""

python3 - "$RESULTS_JSON" <<'PYEOF'
import json, sys

with open(sys.argv[1]) as f:
    data = json.load(f)

results = data["results"]
times = [(r["command"].split()[0], r["median"] * 1000, r["stddev"] * 1000) for r in results]

fastest = min(t for _, t, _ in times)

rules = {"sqrust": 330, "sqruff": "~62", "sqlfluff": "~89"}

header = f"{'Tool':<12} {'Median':>10} {'Std Dev':>10} {'Rules':>8} {'vs fastest':>12}"
print(header)
print("─" * len(header))

for tool, median, stddev in times:
    name = tool.split("/")[-1]
    ratio = median / fastest
    ratio_str = "baseline" if ratio < 1.05 else f"{ratio:.1f}× slower"
    rule_count = rules.get(name, "?")
    print(f"{name:<12} {median:>8.1f}ms {stddev:>8.1f}ms {str(rule_count):>8} {ratio_str:>12}")

print()
fastest_tool = min(times, key=lambda x: x[1])[0].split("/")[-1]
slowest_tool = max(times, key=lambda x: x[1])[0].split("/")[-1]
ratio = max(t for _, t, _ in times) / fastest
print(f"{fastest_tool} is {ratio:.0f}× faster than {slowest_tool} on this corpus.")
PYEOF

# ── caveats ─────────────────────────────────────────────────────────────────
echo ""
echo -e "${YELLOW}Note:${RESET} All tools ran in ANSI dialect mode. Each tool checks different"
echo "rules, so this measures throughput, not rule-for-rule equivalence."
echo "Run on your own dbt project: ./benchmark.sh /path/to/models"
echo ""

# ── cleanup ──────────────────────────────────────────────────────────────────
rm -f "$RESULTS_JSON"
if [[ "$TMPDIR_USED" -eq 1 ]]; then
    rm -rf "$TMP"
fi
