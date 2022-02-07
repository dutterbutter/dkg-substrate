#!/usr/bin/env bash

set -e

# Prepare env flags.
ENV_FLAGS='RUSTC_BOOTSTRAP=1 RUSTFLAGS="-Zinstrument-coverage" CARGO_INCREMENTAL=0 RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort" RUSTDOCFLAGS="-Cpanic=abort"'

# Build project.
eval "$ENV_FLAGS cargo build"

# Run test.
eval "$ENV_FLAGS cargo test"

# Generate code coverage artefacts.
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/

# Open code coverage report in browser.
open target/debug/coverage/index.html
