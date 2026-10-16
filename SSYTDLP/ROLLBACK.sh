#!/bin/sh
set -eu
PACKAGE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python "$PACKAGE_ROOT/rollback/restore.py" "$@"
