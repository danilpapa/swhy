#!/usr/bin/env bash
# E1, «Замер проблемы». Item 2: 500 random Swift lines; share of lines whose commit under `git blame -w` and `git blame -w -M -C -C` is technical.
#
# Prints the item as JSON to stdout. The result file with all four items is
# written by eval/measure/run.sh.
#
#     eval/measure/2-blame-noise.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    measure --repo Signal-iOS --item blame-noise
