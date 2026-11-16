#!/usr/bin/env bash
# E1, «Замер проблемы». Item 3: 200 random methods; share of `git log -L :<func>:<file>` histories that break at a move.
#
# Prints the item as JSON to stdout. The result file with all four items is
# written by eval/measure/run.sh.
#
#     eval/measure/3-history-breaks.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    measure --repo Signal-iOS --item history-breaks
