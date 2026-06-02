#!/usr/bin/env sh
set -eu

: "${HELMBENCH_TASK_ID:?HELMBENCH_TASK_ID is required}"

case "$HELMBENCH_TASK_ID" in
  demo-auth-redirect-001)
    path=src/auth/session.txt
    printf '{"tool":"Read","input":{"path":"%s"}}\n' "$path"
    printf 'expired sessions redirect to /login\nactive sessions redirect to /dashboard\n' > "$path"
    ;;
  demo-billing-rounding-001)
    path=src/billing/invoice.txt
    printf '{"tool":"Read","input":{"path":"%s"}}\n' "$path"
    printf 'invoice rounding mode: round half up\ncurrency: USD\n' > "$path"
    ;;
  *)
    exit 2
    ;;
esac
