#!/bin/sh
set -eu
PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if test -n "${SSMP_PYTHON:-}"; then
    exec "$SSMP_PYTHON" "$PROJECT_ROOT/scripts/rollback.py" "$@"
fi
if command -v python >/dev/null 2>&1; then
    exec python "$PROJECT_ROOT/scripts/rollback.py" "$@"
fi
exec python3 "$PROJECT_ROOT/scripts/rollback.py" "$@"
