#!/usr/bin/env bash
# E1 oracle: builds eval/oracle/methods/signal-ios-<n>.json from the labels
# of the first annotator and, when it exists, eval/oracle/second/*.json from
# the labels of the second annotator. Fails when a step of a trace has no
# label.
#
#     eval/oracle/build.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
first=${SWHY_ORACLE_ANNOTATOR:-agent:claude-fable-5-2}
cargo run --quiet --release --locked -p swhy-eval -- \
    oracle-build --repo Signal-iOS --labels eval/oracle/labels/signal-ios.tsv \
    --annotator "$first" --out-dir eval/oracle/methods
if [ -f eval/oracle/labels/signal-ios-second.tsv ]; then
    : "${SWHY_ORACLE_SECOND_ANNOTATOR:?set SWHY_ORACLE_SECOND_ANNOTATOR to the name of the second annotator}"
    cargo run --quiet --release --locked -p swhy-eval -- \
        oracle-build --repo Signal-iOS --labels eval/oracle/labels/signal-ios-second.tsv \
        --annotator "$SWHY_ORACLE_SECOND_ANNOTATOR" --out-dir eval/oracle/second
fi
