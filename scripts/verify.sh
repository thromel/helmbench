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

test -f LICENSE
grep -q '^license = "MIT"$' Cargo.toml
grep -q 'push:' .github/workflows/release.yml
grep -q 'tags:' .github/workflows/release.yml
grep -q '"v\*"' .github/workflows/release.yml
grep -q 'cp README.md LICENSE' .github/workflows/release.yml
grep -q 'dist/\*.sha256' .github/workflows/release.yml
grep -q 'actions/attest-build-provenance@v2' .github/workflows/release.yml
grep -q 'gh release create' .github/workflows/release.yml
for target in \
  x86_64-unknown-linux-gnu \
  aarch64-apple-darwin \
  x86_64-apple-darwin
do
  grep -q "$target" .github/workflows/release.yml
  grep -q "$target" docs/install.md
done
test -f docs/launch-proof.md
test -f docs/example-benchmark-summary.md
test -f docs/refactoringminer-public-proof.md
test -f docs/refactoringminer-ctxhelm-plan.md
test -f docs/claude-real-smoke.md
test -f reports/claude-real-smoke.json
test -f reports/refactoringminer-suite-health.json
test -f reports/refactoringminer-outcome-health.json
test -f reports/refactoringminer-ctxhelm-plan.json
test -f suites/refactoring-miner-public.json
grep -q '"preset": "claude-code"' suites/demo-matrix.json
grep -q 'preset=claude-code' docs/run-matrix.md
grep -q 'init-public-matrix' README.md
grep -q 'init-public-matrix' docs/refactoringminer-public-proof.md
grep -q 'HelmBench Launch Proof' docs/launch-proof.md
grep -q 'claude-real-smoke' docs/launch-proof.md
grep -q 'claude-real-smoke' docs/direct-agent-runs.md
grep -q 'Low sample size: 1 task' docs/example-benchmark-summary.md
grep -q 'raw source' docs/launch-proof.md
grep -q '"successRate": 1.0' reports/claude-real-smoke.json
grep -q '"sourceFree": true' reports/claude-real-smoke.json
grep -q 'Success rate: `100.0%`' docs/claude-real-smoke.md
grep -q '"ok": true' reports/refactoringminer-suite-health.json
grep -q '"ok": false' reports/refactoringminer-outcome-health.json
grep -q '"validationBaselineReady": false' reports/refactoringminer-outcome-health.json
grep -q '"successCommandCheckFailFast": true' reports/refactoringminer-outcome-health.json
grep -q '"taskCount": 10' reports/refactoringminer-ctxhelm-plan.json
grep -q '"sourceFree": true' reports/refactoringminer-ctxhelm-plan.json
grep -q 'Recommendation recall: `61.3%`' docs/refactoringminer-ctxhelm-plan.md
grep -q 'Average recommendation recall | 61.3%' docs/refactoringminer-public-proof.md

cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings

cargo run -- --help >/dev/null
cargo run -- schema --help >/dev/null
cargo run -- demo-run --help >/dev/null
cargo run -- validate-matrix --help >/dev/null
cargo run -- run-matrix --help >/dev/null
cargo run -- init-public-matrix --help >/dev/null
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
cargo run -- doctor --repo . --format json --out "$TMP_DIR/doctor.json"
grep -q '"ok": true' "$TMP_DIR/doctor.json"
grep -q '"sourceFree": true' "$TMP_DIR/doctor.json"
grep -q '"directRunners"' "$TMP_DIR/doctor.json"

cargo run -- schema --kind task-suite --out "$TMP_DIR/task-suite.schema.json"
cargo run -- schema --kind agent-trace --out "$TMP_DIR/agent-trace.schema.json"
cargo run -- schema --kind agent-event --out "$TMP_DIR/agent-event.schema.json"
cargo run -- schema --kind run-report --out "$TMP_DIR/run-report.schema.json"
cargo run -- schema --kind compare-report --out "$TMP_DIR/compare-report.schema.json"
cargo run -- schema --kind benchmark-summary --out "$TMP_DIR/benchmark-summary.schema.json"
cargo run -- schema --kind quality-gate --out "$TMP_DIR/quality-gate.schema.json"
cargo run -- schema --kind run-matrix-config --out "$TMP_DIR/run-matrix-config.schema.json"
cargo run -- schema --kind matrix-history --out "$TMP_DIR/matrix-history.schema.json"
cargo run -- schema --kind doctor-report --out "$TMP_DIR/doctor-report.schema.json"
cargo run -- schema --kind autopsy --out "$TMP_DIR/autopsy.schema.json"
cargo run -- schema --kind diff-autopsy --out "$TMP_DIR/diff-autopsy.schema.json"
cargo run -- schema --kind suite-health --out "$TMP_DIR/suite-health.schema.json"
cargo run -- schema --kind evidence-bundle --out "$TMP_DIR/evidence-bundle.schema.json"
cargo run -- schema --kind run-matrix-manifest --out "$TMP_DIR/run-matrix-manifest.schema.json"
cargo run -- schema --kind run-matrix-privacy-report --out "$TMP_DIR/run-matrix-privacy-report.schema.json"
cargo run -- schema --all --out-dir "$TMP_DIR/all-schemas"
grep -q '"title": "HelmBench Task Suite"' "$TMP_DIR/task-suite.schema.json"
grep -q '"setupCommands"' "$TMP_DIR/task-suite.schema.json"
grep -q '"title": "HelmBench Agent Trace"' "$TMP_DIR/agent-trace.schema.json"
grep -q '"title": "HelmBench Agent Event"' "$TMP_DIR/agent-event.schema.json"
grep -q '"title": "HelmBench Run Report"' "$TMP_DIR/run-report.schema.json"
grep -q '"title": "HelmBench Compare Report"' "$TMP_DIR/compare-report.schema.json"
grep -q '"title": "HelmBench Benchmark Summary"' "$TMP_DIR/benchmark-summary.schema.json"
grep -q '"title": "HelmBench Quality Gate"' "$TMP_DIR/quality-gate.schema.json"
grep -q '"title": "HelmBench Run Matrix Config"' "$TMP_DIR/run-matrix-config.schema.json"
grep -q '"adapterPreset"' "$TMP_DIR/run-matrix-config.schema.json"
grep -q '"claude-code"' "$TMP_DIR/run-matrix-config.schema.json"
grep -q '"title": "HelmBench Matrix History"' "$TMP_DIR/matrix-history.schema.json"
grep -q '"title": "HelmBench Doctor Report"' "$TMP_DIR/doctor-report.schema.json"
grep -q '"title": "HelmBench Autopsy"' "$TMP_DIR/autopsy.schema.json"
grep -q '"title": "HelmBench Diff Autopsy"' "$TMP_DIR/diff-autopsy.schema.json"
grep -q '"title": "HelmBench Suite Health"' "$TMP_DIR/suite-health.schema.json"
grep -q '"tasksFailedSetupCommand"' "$TMP_DIR/suite-health.schema.json"
grep -q '"title": "HelmBench Evidence Bundle"' "$TMP_DIR/evidence-bundle.schema.json"
grep -q '"title": "HelmBench Run Matrix Manifest"' "$TMP_DIR/run-matrix-manifest.schema.json"
grep -q '"adapterPreset"' "$TMP_DIR/run-matrix-manifest.schema.json"
grep -q '"title": "HelmBench Run Matrix Privacy Report"' "$TMP_DIR/run-matrix-privacy-report.schema.json"
test -f "$TMP_DIR/all-schemas/task-suite.schema.json"
test -f "$TMP_DIR/all-schemas/run-matrix-privacy-report.schema.json"
SCHEMA_COUNT="$(find "$TMP_DIR/all-schemas" -type f -name '*.schema.json' | wc -l | tr -d ' ')"
test "$SCHEMA_COUNT" = "16"

cargo run -- init-demo-repo \
  --repo-out "$TMP_DIR/demo-repo" \
  --suite-out "$TMP_DIR/demo-suite.json" \
  --force

cargo run -- demo-run \
  --out-dir "$TMP_DIR/full-demo" \
  --force

cargo run -- validate-suite "$TMP_DIR/demo-suite.json"
grep -q '"setupCommands"' "$TMP_DIR/demo-suite.json"
(cd "$TMP_DIR/demo-repo" && sh tests/auth/session.test.sh)
(cd "$TMP_DIR/demo-repo" && sh tests/billing/invoice.test.sh)

cargo run -- suite-health \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out "$TMP_DIR/suite-health.json"

cargo run -- suite-health \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out "$TMP_DIR/suite-health.md" \
  --format markdown

cargo run -- suite-health \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out "$TMP_DIR/suite-health-baseline.json" \
  --check-success-commands \
  --fail-fast-success-commands
grep -q '"successCommandCheckRequested": true' "$TMP_DIR/suite-health-baseline.json"
grep -q '"successCommandCheckFailFast": true' "$TMP_DIR/suite-health-baseline.json"
grep -q '"validationBaselineReady": true' "$TMP_DIR/suite-health-baseline.json"
grep -q '"baselineSuccessCommandFailCount": 2' "$TMP_DIR/suite-health-baseline.json"
grep -q '"tasksFailedSetupCommand": \[\]' "$TMP_DIR/suite-health-baseline.json"

cargo run -- suite-health \
  --suite suites/local-run-smoke.json \
  --repo . \
  --out "$TMP_DIR/local-run-smoke-health-baseline.json" \
  --allow-dirty \
  --check-success-commands
grep -q '"successCommandCheckRequested": true' "$TMP_DIR/local-run-smoke-health-baseline.json"
grep -q '"validationBaselineReady": true' "$TMP_DIR/local-run-smoke-health-baseline.json"
grep -q '"baselineSuccessCommandFailCount": 1' "$TMP_DIR/local-run-smoke-health-baseline.json"
grep -q '"tasksFailedSetupCommand": \[\]' "$TMP_DIR/local-run-smoke-health-baseline.json"

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

cat > "$TMP_DIR/fake-ctxhelm.sh" <<'EOF'
#!/usr/bin/env sh
set -eu

case "${1:-}" in
  prepare-task)
    printf '{"targetFiles":[{"path":"src/auth/session.txt"}],"relatedTests":[{"path":"auth.test"}]}\n'
    ;;
  get-pack)
    printf '{"tokenEstimate":321,"sections":[]}\n'
    ;;
  *)
    exit 2
    ;;
esac
EOF
chmod +x "$TMP_DIR/fake-ctxhelm.sh"

cargo run -- run-matrix \
  --suite "$TMP_DIR/demo-suite.json" \
  --repo "$TMP_DIR/demo-repo" \
  --out-dir "$TMP_DIR/matrix" \
  --baseline "name=native,agent=demo-baseline,variant=native" \
  --head "name=native-search,agent=demo-native-search,variant=native_search,command=HELMBENCH_BIN=$ROOT/target/debug/helmbench sh scripts/demo-agent.sh" \
  --head "name=guided,agent=demo-guided,variant=ctxhelm_mcp,ctxhelm=true,ctxhelm_bin=$TMP_DIR/fake-ctxhelm.sh,pack=true,pack_budget=brief,command=HELMBENCH_BIN=$ROOT/target/debug/helmbench sh scripts/demo-agent.sh" \
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
      "name": "native-search",
      "agent": "demo-native-search",
      "variant": "native_search",
      "command": "HELMBENCH_BIN=\${HELMBENCH_BIN:?set HELMBENCH_BIN} sh scripts/demo-agent.sh"
    },
    {
      "name": "guided",
      "agent": "demo-guided",
      "variant": "ctxhelm_mcp",
      "ctxhelm": true,
      "ctxhelmBin": "$TMP_DIR/fake-ctxhelm.sh",
      "pack": true,
      "packBudget": "brief",
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

HELMBENCH_BIN="$ROOT/target/debug/helmbench" cargo run -- validate-matrix \
  --config suites/demo-matrix.json

HELMBENCH_BIN="$ROOT/target/debug/helmbench" cargo run -- run-matrix \
  --config suites/demo-matrix.json \
  --out-dir "$TMP_DIR/checked-in-matrix" \
  --force \
  --allow-dirty-health

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

cargo run -- verify-bundle \
  --bundle "$TMP_DIR/checked-in-matrix/evidence"

cargo run -- verify-matrix \
  --matrix "$TMP_DIR/checked-in-matrix"

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
test -f "$TMP_DIR/task-suite.schema.json"
test -f "$TMP_DIR/agent-trace.schema.json"
test -f "$TMP_DIR/agent-event.schema.json"
test -f "$TMP_DIR/run-report.schema.json"
test -f "$TMP_DIR/compare-report.schema.json"
test -f "$TMP_DIR/benchmark-summary.schema.json"
test -f "$TMP_DIR/quality-gate.schema.json"
test -f "$TMP_DIR/run-matrix-config.schema.json"
test -f "$TMP_DIR/matrix-history.schema.json"
test -f "$TMP_DIR/doctor-report.schema.json"
test -f "$TMP_DIR/autopsy.schema.json"
test -f "$TMP_DIR/diff-autopsy.schema.json"
test -f "$TMP_DIR/suite-health.schema.json"
test -f "$TMP_DIR/evidence-bundle.schema.json"
test -f "$TMP_DIR/run-matrix-manifest.schema.json"
test -f "$TMP_DIR/run-matrix-privacy-report.schema.json"
test -f "$TMP_DIR/all-schemas/task-suite.schema.json"
test -f "$TMP_DIR/all-schemas/run-matrix-privacy-report.schema.json"
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
test -f "$TMP_DIR/matrix/reports/native-search.json"
test -f "$TMP_DIR/matrix/reports/compare-native-search.json"
test -f "$TMP_DIR/matrix/reports/guided.json"
test -f "$TMP_DIR/matrix/reports/quality-gate.json"
test -f "$TMP_DIR/matrix/reports/privacy-report.json"
test -f "$TMP_DIR/matrix/docs/dashboard.html"
test -f "$TMP_DIR/matrix/docs/compare-native-search.md"
test -f "$TMP_DIR/matrix/docs/native-search-autopsy.md"
test -f "$TMP_DIR/matrix/docs/privacy-report.md"
test -f "$TMP_DIR/matrix/docs/guided-autopsy.md"
test -f "$TMP_DIR/matrix/docs/reproduction.md"
test -f "$TMP_DIR/matrix/evidence/health.json"
test -f "$TMP_DIR/matrix/evidence/manifest.json"
test -f "$TMP_DIR/matrix/matrix-manifest.json"
test -f "$TMP_DIR/matrix-config/reports/benchmark-summary.json"
test -f "$TMP_DIR/matrix-config/reports/suite-health.json"
test -f "$TMP_DIR/matrix-config/reports/native-search.json"
test -f "$TMP_DIR/matrix-config/reports/compare-native-search.json"
test -f "$TMP_DIR/matrix-config/reports/guided.json"
test -f "$TMP_DIR/matrix-config/reports/quality-gate.json"
test -f "$TMP_DIR/matrix-config/reports/privacy-report.json"
test -f "$TMP_DIR/matrix-config/docs/compare-native-search.md"
test -f "$TMP_DIR/matrix-config/docs/native-search-autopsy.md"
test -f "$TMP_DIR/matrix-config/docs/privacy-report.md"
test -f "$TMP_DIR/matrix-config/docs/guided-autopsy.md"
test -f "$TMP_DIR/matrix-config/docs/reproduction.md"
test -f "$TMP_DIR/matrix-config/evidence/health.json"
test -f "$TMP_DIR/matrix-config/evidence/manifest.json"
test -f "$TMP_DIR/matrix-config/matrix-manifest.json"
test -f "$TMP_DIR/checked-in-matrix/reports/native-search.json"
test -f "$TMP_DIR/checked-in-matrix/reports/guided.json"
test -f "$TMP_DIR/checked-in-matrix/reports/privacy-report.json"
test -f "$TMP_DIR/checked-in-matrix/docs/reproduction.md"
test -f "$TMP_DIR/checked-in-matrix/evidence/manifest.json"
test -f "$TMP_DIR/checked-in-matrix/matrix-manifest.json"

grep -q '"ctxhelmEnabled": true' "$TMP_DIR/matrix/matrix-manifest.json"
grep -q '"packEnabled": true' "$TMP_DIR/matrix/matrix-manifest.json"
grep -q '"ctxhelmEnabled": true' "$TMP_DIR/matrix-config/matrix-manifest.json"
grep -q '"packEnabled": true' "$TMP_DIR/matrix-config/matrix-manifest.json"
grep -q '"ctxhelmEnabled": true' "$TMP_DIR/checked-in-matrix/matrix-manifest.json"
grep -q '"packEnabled": true' "$TMP_DIR/checked-in-matrix/matrix-manifest.json"
grep -q '"adapterPreset": "claude-code"' "$TMP_DIR/checked-in-matrix/matrix-manifest.json"
grep -q '"totalTokenEstimate": 642' "$TMP_DIR/matrix/reports/guided.json"
grep -q '"totalTokenEstimate": 642' "$TMP_DIR/matrix-config/reports/guided.json"
grep -q '"totalTokenEstimate": 64' "$TMP_DIR/checked-in-matrix/reports/guided.json"

git diff --check

printf 'HelmBench verification passed\n'
