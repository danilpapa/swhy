#!/usr/bin/env bash
# E1 oracle, selection pool: a random sample of Signal-iOS methods with the
# shape of their `git log -L :<func>:<file>` history. Selection rules are in
# docs/oracle-guide.md, «Отбор методов».
#
# Writes target/eval/E1/pool.json (not committed: it is reproducible).
#
#     eval/oracle/pool.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    oracle-pool --repo Signal-iOS --out target/eval/E1/pool.json
