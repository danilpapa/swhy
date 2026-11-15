#!/usr/bin/env bash
# E1, «Замер проблемы». Item 1: commits, years, Swift and ObjC files, commits that change more than 1000 files.
#
# Prints the item as JSON to stdout. The result file with all four items is
# written by eval/measure/run.sh.
#
#     eval/measure/1-repo-stats.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    measure --repo Signal-iOS --item repo-stats
