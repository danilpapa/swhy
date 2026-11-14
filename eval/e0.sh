#!/usr/bin/env bash
# E0 acceptance criterion: «fixtures::build дважды даёт одинаковые SHA».
#
# Builds every fixture twice from scratch and writes
# eval/results/E0-<date>.json (head_sha, command, params, metrics).
# Exit code 0 when all builds are identical, 1 otherwise.
#
#     eval/e0.sh
set -euo pipefail

cd "$(dirname "$0")/.."
out="eval/results/E0-$(date +%Y-%m-%d).json"
cargo run --quiet --locked -p swhy-eval -- \
    fixture-determinism --builds 2 --out "$out" --command "eval/e0.sh"
