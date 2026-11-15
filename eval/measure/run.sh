#!/usr/bin/env bash
# E1, «Замер проблемы (N6, до появления инструмента)»: items 1-4 on
# Signal-iOS at the commit pinned in eval/repos.toml.
#
# Writes eval/results/E1-measure.json (head_sha, command, params, metrics).
# Reads the clone in data/repos/Signal-iOS, never writes to it, no network.
#
#     eval/measure/run.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    measure --repo Signal-iOS \
    --out eval/results/E1-measure.json --command "eval/measure/run.sh"
