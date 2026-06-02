#!/usr/bin/env sh
set -eu

: "${HELMBENCH_BIN:=helmbench}"
: "${HELMBENCH_EVENTS:?HELMBENCH_EVENTS is required}"
: "${HELMBENCH_TASK_ID:?HELMBENCH_TASK_ID is required}"

"$HELMBENCH_BIN" record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind recommended-file \
  --path examples/demo-app/auth.txt \
  --observed-at-millis 10

"$HELMBENCH_BIN" record-event \
  --events "$HELMBENCH_EVENTS" \
  --task-id "$HELMBENCH_TASK_ID" \
  --event-kind file-read \
  --path examples/demo-app/auth.txt \
  --observed-at-millis 20

printf 'fixed sessions redirect to /login\n' > examples/demo-app/auth.txt
