#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="wasm32-unknown-unknown"
CRATE="c2-ecs-wasm"
OUT_DIR="${ROOT_DIR}/services/c2-web/static/wasm"
RAW_OUT="${OUT_DIR}/c2-ecs-wasm.raw.wasm"
FINAL_OUT="${OUT_DIR}/c2-ecs-wasm.wasm"

if ! command -v wasm-opt >/dev/null 2>&1; then
  echo "wasm-opt is required (install binaryen) to build optimized WASM." >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -q "${TARGET}"; then
    rustup target add "${TARGET}"
  fi
fi

cargo build -p "${CRATE}" --target "${TARGET}" --release --manifest-path "${ROOT_DIR}/Cargo.toml"
mkdir -p "${OUT_DIR}"
cp "${ROOT_DIR}/target/${TARGET}/release/c2_ecs_wasm.wasm" "${RAW_OUT}"
wasm-opt -Oz --strip-debug -o "${FINAL_OUT}" "${RAW_OUT}"
rm -f "${RAW_OUT}"
