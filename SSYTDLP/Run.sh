#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")"
if ! test -f .venv/bin/python; then
    python3 -m venv .venv
    .venv/bin/python -m pip install -r source/requirements.txt
fi
if ! test -f tools/yt-dlp; then
    .venv/bin/python source/prepare_tools.py
fi
exec .venv/bin/python source/app.py
