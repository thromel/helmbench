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
cargo run -- demo-run --help >/dev/null
cargo run -- validate-matrix --help >/dev/null
cargo run -- run-matrix --help >/dev/null
cargo run -- matrix-history --help >/dev/null
cargo run -- init-public-suite --help >/dev/null
cargo run -- suite-health --help >/dev/null
cargo run -- benchmark-summary --help >/dev/null
cargo run -- evidence-bundle --help >/dev/null
cargo run -- verify-bundle --help >/dev/null
cargo run -- verify-matrix --help >/dev/null
cargo run -- quality-gate --help >/dev/null
cargo run -- autopsy --help >/dev/null
cargo run -- diff-autopsy --help >/dev/null
cargo run -- dashboard --help >/dev/null
cargo run -- doctor --repo . >/dev/null

cargo run -- init-demo-repo \
  --repo-out "$TMP_DIR/demo-repo" \
  --suite-out "$TMP_DIR/demo-suite.json" \
  --force

cargo run -- demo-run \
  --out-dir "$TMP_DIR/full-demo" \
  --force

cargo run -- validate-suite "$TMP_DIR/demo-suite.json"

cargo run -- suite-health \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out "$TMP_DIR/suite-health.json"

cargo run -- suite-health \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out "$TMP_DIR/suite-health.md" \
  --format markdown

cargo run -- local-run \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --work-dir "$TMP_DIR/workdirs" \
  --out-dir "$TMP_DIR/traces" \
  --adapter-command "HELMBENCH_BIN=$ROOT/target/debug/helmbench sh scripts/demo-agent.sh"

cargo run -- local-run \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --work-dir "$TMP_DIR/stream-workdirs" \
  --out-dir "$TMP_DIR/stream-traces" \
  --adapter-command "sh $ROOT/scripts/demo-stream-agent.sh" \
  --capture-stream

cargo run -- run-matrix \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out-dir "$TMP_DIR/matrix" \
  --baseline "name=native,agent=demo-baseline,variant=native" \
  --head "name=guided,agent=demo-guided,variant=ctxhelm_mcp,command=HELMBENCH_BIN=$ROOT/target/debug/helmbench sh scripts/demo-agent.sh" \
  --force

cat > "$TMP_DIR/matrix-config.json" <<EOF
{
  "suite": "$TMP_DIR/demo-suite.json",
  "repo": "$TMP_DIR/demo-repo",
  "outDir": "$TMP_DIR/matrix-config",
  "failOnRegression": true,
  "baseline": {
    "name": "native",
    "agent": "demo-baseline",
    "variant": "native"
  },
  "heads": [
    {
      "name": "guided",
      "agent": "demo-guided",
      "variant": "ctxhelm_mcp",
      "command": "HELMBENCH_BIN=\${HELMBENCH_BIN:?set HELMBENCH_BIN} sh scripts/demo-agent.sh"
    }
  ]
}
EOF

cargo run -- validate-matrix \
  --config "$TMP_DIR/matrix-config.json"

HELMBENCH_BIN="$ROOT/target/debug/helmbench" cargo run -- run-matrix \
  --config "$TMP_DIR/matrix-config.json" \
  --force

cargo run -- run \
  --suite "$TMP_DIR/demo-suite.json" \
  --trace-dir "$TMP_DIR/traces" \
  --out "$TMP_DIR/report.json"

cargo run -- run \
  --suite "$TMP_DIR/demo-suite.json" \
  --trace-dir "$TMP_DIR/stream-traces" \
  --out "$TMP_DIR/stream-report.json"

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

cargo run -- benchmark-summary \
  --base reports/example-native.json \
  --head reports/example-ctxhelm.json \
  --head reports/example-claude-code.json \
  --out "$TMP_DIR/benchmark-summary.json" \
  --format json

cargo run -- quality-gate \
  --summary "$TMP_DIR/benchmark-summary.json" \
  --out "$TMP_DIR/quality-gate.md" \
  --max-total-tool-calls-delta 0 \
  --max-total-token-estimate-delta 0 \
  --max-tool-calls-per-success-delta 0 \
  --max-token-estimate-per-success-delta 0

cargo run -- evidence-bundle \
  --suite suites/example-auth-bugs.json \
  --base-report reports/example-native.json \
  --head-report reports/example-ctxhelm.json \
  --head-report reports/example-claude-code.json \
  --out-dir "$TMP_DIR/evidence" \
  --force

cargo run -- verify-bundle \
  --bundle "$TMP_DIR/evidence"

cargo run -- verify-bundle \
  --bundle "$TMP_DIR/full-demo/evidence"

cargo run -- verify-bundle \
  --bundle "$TMP_DIR/matrix/evidence"

cargo run -- verify-matrix \
  --matrix "$TMP_DIR/matrix"

cargo run -- verify-bundle \
  --bundle "$TMP_DIR/matrix-config/evidence"

cargo run -- verify-matrix \
  --matrix "$TMP_DIR/matrix-config"

cargo run -- matrix-history \
  --matrix "$TMP_DIR/matrix" \
  --matrix "$TMP_DIR/matrix-config" \
  --out "$TMP_DIR/matrix-history.md"

cargo run -- matrix-history \
  --matrix "$TMP_DIR/matrix" \
  --matrix "$TMP_DIR/matrix-config" \
  --format json \
  --out "$TMP_DIR/matrix-history.json"

cargo run -- matrix-history \
  --matrix "$TMP_DIR/matrix" \
  --matrix "$TMP_DIR/matrix-config" \
  --format html \
  --out "$TMP_DIR/matrix-history.html"

printf '\n# fixed redirect\n' >> "$TMP_DIR/demo-repo/src/auth/session.txt"
printf '\n# regression coverage\n' >> "$TMP_DIR/demo-repo/tests/auth/session.test.sh"

cargo run -- diff-autopsy \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --task-id demo-auth-redirect-001 \
  --out "$TMP_DIR/diff-autopsy.json" \
  --format json

cargo run -- diff-autopsy \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --task-id demo-auth-redirect-001 \
  --out "$TMP_DIR/diff-autopsy.md" \
  --format markdown

test -f "$TMP_DIR/report.json"
test -f "$TMP_DIR/stream-report.json"
test -f "$TMP_DIR/autopsy.md"
test -f "$TMP_DIR/diff-autopsy.json"
test -f "$TMP_DIR/diff-autopsy.md"
test -f "$TMP_DIR/dashboard.html"
test -f "$TMP_DIR/suite-health.json"
test -f "$TMP_DIR/suite-health.md"
test -f "$TMP_DIR/benchmark-summary.md"
test -f "$TMP_DIR/benchmark-summary.json"
test -f "$TMP_DIR/quality-gate.md"
test -f "$TMP_DIR/matrix-history.md"
test -f "$TMP_DIR/matrix-history.json"
test -f "$TMP_DIR/matrix-history.html"
test -f "$TMP_DIR/evidence/manifest.json"
test -f "$TMP_DIR/full-demo/evidence/manifest.json"
test -f "$TMP_DIR/full-demo/docs/dashboard.html"
test -f "$TMP_DIR/matrix/reports/benchmark-summary.json"
test -f "$TMP_DIR/matrix/reports/suite-health.json"
test -f "$TMP_DIR/matrix/reports/quality-gate.json"
test -f "$TMP_DIR/matrix/docs/dashboard.html"
test -f "$TMP_DIR/matrix/docs/guided-autopsy.md"
test -f "$TMP_DIR/matrix/docs/reproduction.md"
test -f "$TMP_DIR/matrix/evidence/health.json"
test -f "$TMP_DIR/matrix/evidence/manifest.json"
test -f "$TMP_DIR/matrix/matrix-manifest.json"
test -f "$TMP_DIR/matrix-config/reports/benchmark-summary.json"
test -f "$TMP_DIR/matrix-config/reports/suite-health.json"
test -f "$TMP_DIR/matrix-config/reports/quality-gate.json"
test -f "$TMP_DIR/matrix-config/docs/guided-autopsy.md"
test -f "$TMP_DIR/matrix-config/docs/reproduction.md"
test -f "$TMP_DIR/matrix-config/evidence/health.json"
test -f "$TMP_DIR/matrix-config/evidence/manifest.json"
test -f "$TMP_DIR/matrix-config/matrix-manifest.json"

git diff --check

printf 'HelmBench verification passed\n'
