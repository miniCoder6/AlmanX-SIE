#!/usr/bin/env bash
# ─── AlmanX: Seed Synthetic Test Data ────────────────────────────────────────
# Usage: bash integration_tests/seed_data.sh [path-to-almanx-binary]
# Seeds ~100 realistic commands to test all features immediately.

set -euo pipefail

ALMANX="${1:-./target/release/almanx}"

if [ ! -x "$ALMANX" ]; then
    echo "Error: '$ALMANX' not found or not executable."
    echo "Build first with: cargo build --release"
    exit 1
fi

echo "Seeding AlmanX with synthetic data using: $ALMANX"
echo ""

CWD_PROJ="$HOME/projects/myapp"
CWD_INFRA="$HOME/projects/infra"

# ── Git workflow (repeated 10 times to trigger workflow mining) ───────────────
echo "→ Seeding git workflow..."
for i in $(seq 1 10); do
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 50  git status
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 100 git add .
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 300 git commit -m "update"
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 2000 git push
done

# ── Cargo workflow ────────────────────────────────────────────────────────────
echo "→ Seeding cargo workflow..."
for i in $(seq 1 8); do
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 8000  cargo build --release
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 5000  cargo test
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 1500  cargo clippy
done

# ── Docker workflow ───────────────────────────────────────────────────────────
echo "→ Seeding docker workflow..."
for i in $(seq 1 5); do
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 45000 docker build -t myapp .
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 800   docker run -p 8080:8080 myapp
    "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 200   docker ps
done

# ── Misc commands ─────────────────────────────────────────────────────────────
echo "→ Seeding misc commands..."
for cmd in "ls -la" "cd .." "cat README.md" "vim src/main.rs" "kubectl get pods" "npm install" "npm run build" "python3 -m pytest"; do
    for i in $(seq 1 3); do
        "$ALMANX" record --cwd "$CWD_PROJ" --exit-code 0 --duration 100 $cmd
    done
done

# ── Some failed commands (for stats success rate) ────────────────────────────
"$ALMANX" record --cwd "$CWD_PROJ" --exit-code 1 --duration 200 cargo build 2>/dev/null || true
"$ALMANX" record --cwd "$CWD_PROJ" --exit-code 1 --duration 50  git push    2>/dev/null || true

echo ""
echo "✓ Done! Data seeded. Try:"
echo "  $ALMANX stats"
echo "  $ALMANX workflows"
echo "  $ALMANX search git"
echo "  $ALMANX suggest"
