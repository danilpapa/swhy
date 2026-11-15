#!/usr/bin/env bash
# E1 acceptance criterion «50 размеченных методов, каппа посчитана»:
# validates eval/oracle/methods/*.json against docs/TZ.md (50 methods, 25/25
# split, at least 3 changing commits each, at least 10 with a module move and
# 10 with ObjC origin) and computes Cohen's kappa against the second
# annotator in eval/oracle/second/*.json.
#
# Writes eval/results/E1-<date>.json. Exit code 0 when every requirement
# holds, 1 otherwise (the violations are listed in metrics.violations).
#
#     eval/oracle/check.sh
set -euo pipefail

cd "$(dirname "$0")/../.."
out="eval/results/E1-$(date +%Y-%m-%d).json"
cargo run --quiet --locked -p swhy-eval -- \
    oracle-check --out "$out" --command "eval/oracle/check.sh"
