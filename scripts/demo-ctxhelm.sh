#!/usr/bin/env sh
set -eu

case "${1:-}" in
  prepare-task)
    printf '{"targetFiles":[{"path":"examples/demo-app/auth.txt"}],"relatedTests":[{"path":"examples/demo-app/auth.test"}]}\n'
    ;;
  get-pack)
    printf '{"tokenEstimate":64,"sections":[]}\n'
    ;;
  *)
    exit 2
    ;;
esac
