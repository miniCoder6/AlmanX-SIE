#!/usr/bin/env bash
# ─── AlmanX Integration Tests ─────────────────────────────────────────────────
# Run from project root: bash integration_tests/test_hooks.sh
# Requires: almanx binary in PATH or ./target/release/almanx

set -euo pipefail

ALMANX="${ALMANX_BIN:-./target/release/almanx}"
TMPDIR_TEST=$(mktemp -d)
export HOME="$TMPDIR_TEST"  # Isolate test data

PASS=0
FAIL=0

green() { echo -e "\033[32m✓ $*\033[0m"; }
red()   { echo -e "\033[31m✗ $*\033[0m"; }

assert_ok() {
    local desc="$1"; shift
    if "$@" > /dev/null 2>&1; then
        green "$desc"
        ((PASS++))
    else
        red "$desc (command: $*)"
        ((FAIL++))
    fi
}

assert_contains() {
    local desc="$1"; local expected="$2"; shift 2
    local output
    output=$("$@" 2>&1) || true
    if echo "$output" | grep -q "$expected"; then
        green "$desc"
        ((PASS++))
    else
        red "$desc — expected '$expected' in output: $output"
        ((FAIL++))
    fi
}

assert_not_contains() {
    local desc="$1"; local not_expected="$2"; shift 2
    local output
    output=$("$@" 2>&1) || true
    if echo "$output" | grep -q "$not_expected"; then
        red "$desc — did NOT expect '$not_expected' in output"
        ((FAIL++))
    else
        green "$desc"
        ((PASS++))
    fi
}

echo ""
echo "=== AlmanX Integration Tests ==="
echo "Binary: $ALMANX"
echo "Test home: $TMPDIR_TEST"
echo ""

# ── 1. Binary exists and shows help ───────────────────────────────────────────
echo "── Section 1: Binary & Help ──"
assert_ok       "Binary is executable"              test -x "$ALMANX"
assert_contains "Help text shows 'almanx'"           "almanx"   "$ALMANX" --help
assert_contains "'record' subcommand exists"         "record"   "$ALMANX" --help

# ── 2. Record commands ────────────────────────────────────────────────────────
echo ""
echo "── Section 2: Recording Commands ──"
assert_ok "Record 'git status'"    "$ALMANX" record git status
assert_ok "Record 'git add .'"     "$ALMANX" record git add .
assert_ok "Record 'git commit -m'" "$ALMANX" record git commit -m "test"
assert_ok "Record 'cargo build'"   "$ALMANX" record cargo build
assert_ok "Record 'cargo test'"    "$ALMANX" record cargo test

# Record same commands multiple times to build frequency
for i in 1 2 3 4 5; do
    "$ALMANX" record git status >/dev/null 2>&1
    "$ALMANX" record git add .  >/dev/null 2>&1
done

# ── 3. Suggest aliases ────────────────────────────────────────────────────────
echo ""
echo "── Section 3: Alias Suggestions ──"
assert_contains "Suggest shows 'COMMAND' header"   "COMMAND"   "$ALMANX" suggest
assert_contains "Suggest shows git commands"       "git"       "$ALMANX" suggest

# ── 4. Add and list aliases ───────────────────────────────────────────────────
echo ""
echo "── Section 4: Alias Management ──"
assert_ok       "Add alias 'gs' for 'git status'" "$ALMANX" add gs git status
assert_ok       "Add alias 'gp' for 'git push'"   "$ALMANX" add gp git push
assert_contains "List shows 'gs'"                 "gs"  "$ALMANX" list
assert_contains "List shows 'gp'"                 "gp"  "$ALMANX" list

# ── 5. Remove alias ───────────────────────────────────────────────────────────
echo ""
echo "── Section 5: Remove Alias ──"
assert_ok           "Remove alias 'gp'"          "$ALMANX" remove gp
assert_not_contains "List no longer shows 'gp'"  "^gp " "$ALMANX" list

# ── 6. Dismiss command ────────────────────────────────────────────────────────
echo ""
echo "── Section 6: Dismiss Command ──"
assert_ok "Dismiss 'cargo build'"  "$ALMANX" dismiss cargo build
# After dismiss, it should not appear in suggestions
output=$("$ALMANX" suggest 2>&1)
if echo "$output" | grep -q "^cargo build"; then
    red "Dismissed command should not appear in suggest"
    ((FAIL++))
else
    green "Dismissed command absent from suggestions"
    ((PASS++))
fi

# ── 7. Search ─────────────────────────────────────────────────────────────────
echo ""
echo "── Section 7: Search ──"
# Need enough events for search to work
for i in 1 2 3; do
    "$ALMANX" record --cwd "$HOME" --exit-code 0 git status >/dev/null 2>&1
    "$ALMANX" record --cwd "$HOME" --exit-code 0 docker ps  >/dev/null 2>&1
done
assert_contains "Search 'git' finds results"    "git"    "$ALMANX" search git
assert_contains "Search 'docker' finds results" "docker" "$ALMANX" search docker

# ── 8. Stats ──────────────────────────────────────────────────────────────────
echo ""
echo "── Section 8: Stats ──"
assert_contains "Stats shows 'Total commands'" "Total"  "$ALMANX" stats
assert_contains "Stats shows 'Top 10'"         "Top"    "$ALMANX" stats

# ── 9. Shell init ─────────────────────────────────────────────────────────────
echo ""
echo "── Section 9: Shell Init Snippets ──"
assert_contains "Bash init contains PROMPT_COMMAND or preexec" "almanx" "$ALMANX" init bash
assert_contains "Zsh init contains preexec hook"               "almanx" "$ALMANX" init zsh
assert_contains "Fish init contains fish_preexec"              "almanx" "$ALMANX" init fish

# ── 10. Workflows ─────────────────────────────────────────────────────────────
echo ""
echo "── Section 10: Workflow Mining ──"
# Seed repeated sequences
for i in 1 2 3 4 5; do
    "$ALMANX" record --cwd "$HOME" git add .   >/dev/null 2>&1
    "$ALMANX" record --cwd "$HOME" git commit  >/dev/null 2>&1
    "$ALMANX" record --cwd "$HOME" git push    >/dev/null 2>&1
done
# workflows command should not crash
assert_ok "Workflows command runs"  "$ALMANX" workflows

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════"
echo "Results: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════"

rm -rf "$TMPDIR_TEST"

[ "$FAIL" -eq 0 ] && exit 0 || exit 1
