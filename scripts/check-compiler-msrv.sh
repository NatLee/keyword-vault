#!/usr/bin/env bash
set -euo pipefail
# Run with Rust 1.74 selected (CI) or inside the official rust:1.74 image.
repo_dir=$(cd "$(dirname "$0")/.." && pwd)
compiler_tmp=$(mktemp -d)
trap 'rm -rf "$compiler_tmp"' EXIT
cp -R "$repo_dir/crates/es-keyword-query/." "$compiler_tmp/"
rm -rf "$compiler_tmp/target" "$compiler_tmp/Cargo.lock"
cd "$compiler_tmp"
rustc --version | awk '$2 ~ /^1\.74\./ { found=1 } END { exit !found }'
CARGO_BUILD_JOBS=2 cargo test --lib --tests
printf '%s\n' 'Standalone compiler MSRV 1.74 passed'
