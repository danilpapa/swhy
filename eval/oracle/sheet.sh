#!/usr/bin/env bash
# E1 oracle, review sheet of one method: candidate commits (`git log -L`,
# `git log --follow`, merges) with the diff of the method in each.
#
#     eval/oracle/sheet.sh <path> <key> [--cut <commit>]...
#     eval/oracle/sheet.sh Signal/Foo.swift 'swift:Foo.bar(x:)'
set -euo pipefail

path=$1
key=$2
shift 2
cd "$(dirname "$0")/../.."
cargo run --quiet --release --locked -p swhy-eval -- \
    oracle-sheet --repo Signal-iOS --path "$path" --key "$key" "$@"
