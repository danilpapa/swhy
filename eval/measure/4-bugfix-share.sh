#!/usr/bin/env bash
# E1, «Замер проблемы». Item 4: rough share of bugfix commits by keywords.
#
# Prints the item as JSON to stdout. The result file with all four items is
# written by eval/measure/run.sh.
#
#     eval/measure/4-bugfix-share.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    measure --repo Signal-iOS --item bugfix-share
