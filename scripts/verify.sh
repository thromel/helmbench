#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

TMP_DIR="${TMPDIR:-/tmp}/helmbench-verify-$$"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

mkdir -p "$TMP_DIR"

cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings

cargo run -- --help >/dev/null
cargo run -- init-public-suite --help >/dev/null
cargo run -- benchmark-summary --help >/dev/null
cargo run -- evidence-bundle --help >/dev/null
cargo run -- doctor --repo . >/dev/null

cargo run -- init-demo-repo \
  --repo-out "$TMP_DIR/demo-repo" \
  --suite-out "$TMP_DIR/demo-suite.json" \
  --force

cargo run -- validate-suite "$TMP_DIR/demo-suite.json"

cargo run -- local-run \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --work-dir "$TMP_DIR/workdirs" \
  --out-dir "$TMP_DIR/traces" \
  --adapter-command "HELMBENCH_BIN=$ROOT/target/debug/helmbench sh scripts/demo-agent.sh"

cargo run -- run \
  --suite "$TMP_DIR/demo-suite.json" \
  --trace-dir "$TMP_DIR/traces" \
  --out "$TMP_DIR/report.json"

cargo run -- autopsy \
  --suite "$TMP_DIR/demo-suite.json" \
  --trace-dir "$TMP_DIR/traces" \
  --out "$TMP_DIR/autopsy.md"

cargo run -- dashboard \
  --report reports/example-native.json \
  --report reports/example-ctxhelm.json \
  --report reports/example-claude-code.json \
  --out "$TMP_DIR/dashboard.html"

cargo run -- benchmark-summary \
  --base reports/example-native.json \
  --head reports/example-ctxhelm.json \
  --head reports/example-claude-code.json \
  --out "$TMP_DIR/benchmark-summary.md" \
  --format markdown

cargo run -- evidence-bundle \
  --suite suites/example-auth-bugs.json \
  --base-report reports/example-native.json \
  --head-report reports/example-ctxhelm.json \
  --head-report reports/example-claude-code.json \
  --out-dir "$TMP_DIR/evidence" \
  --force

test -f "$TMP_DIR/report.json"
test -f "$TMP_DIR/autopsy.md"
test -f "$TMP_DIR/dashboard.html"
test -f "$TMP_DIR/benchmark-summary.md"
test -f "$TMP_DIR/evidence/manifest.json"

git diff --check

printf 'HelmBench verification passed\n'
