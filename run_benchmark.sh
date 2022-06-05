#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
# set -o xtrace

ROOT_DIR=$(dirname "$0")

LOX=${LOX:-${ROOT_DIR}/target/release/rlox-interpreter}

for SCRIPT_PATH in "${ROOT_DIR}"/resources/benchmark/*.lox; do
    SCRIPT=$(basename "${SCRIPT_PATH}" | tr -d "\n");
    TIMES=()
    for _ in {1..3}; do
        TIME=$("${LOX}" "${SCRIPT_PATH}" | grep "^elapsed:$" --after-context 1 | tail --lines 1 | tr -d "\n");
        TIMES+=("${TIME}")
    done
    MIN_TIME=$(printf "%s\n" "${TIMES[@]}" | LC_ALL=C sort --numeric-sort | head --lines 1 | tr -d '\n');
    echo "${SCRIPT},${MIN_TIME}";
done

