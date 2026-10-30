#!/bin/sh
# Open or focus project Spaces, each with its own task input and visible agent terminals.
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec sh "$SCRIPT_DIR/../../bin/run.sh" "${1:-control-open}"
