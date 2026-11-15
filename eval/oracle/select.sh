#!/usr/bin/env bash
# E1 oracle, selection: traces the first pool methods of every stratum,
# takes the first eligible ones (12 with ObjC origin, 12 with a module move,
# 26 others) and writes for the annotator, into target/eval/E1/:
#   screen.json    every traced method and why it was or was not eligible
#   skeleton.tsv   a labels file with `?` where a class must be decided
#   sheets/<id>.txt review sheets
# Methods listed in eval/oracle/rejected.tsv are skipped. Rules are in
# docs/oracle-guide.md, «Отбор методов». Needs eval/oracle/pool.sh first.
#
#     eval/oracle/select.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    oracle-select --repo Signal-iOS --pool target/eval/E1/pool.json \
    --skip eval/oracle/rejected.tsv --out-dir target/eval/E1 "$@"
