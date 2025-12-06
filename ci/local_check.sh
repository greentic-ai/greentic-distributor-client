#!/usr/bin/env bash
set -euo pipefail

echo ">> fmt"
cargo fmt --all -- --check

echo ">> clippy"
cargo clippy --all-targets --all-features -- -D warnings

echo ">> tests"
cargo test --all-features
