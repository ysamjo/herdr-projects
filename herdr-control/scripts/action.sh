#!/bin/sh
# Navigate a project's existing terminals through its own source build.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec sh "$SCRIPT_DIR/../../bin/run.sh" "$@"
