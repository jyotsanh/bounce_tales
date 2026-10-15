#!/usr/bin/env bash
# Format, lint and test. Run before handing off every step.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet
