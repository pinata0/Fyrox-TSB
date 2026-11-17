#!/usr/bin/env bash
set -euo pipefail
cd /workspace
mkdir -p artifacts/linux-build target/linux-x64/release
cargo build --locked --release --bins 2>&1 | tee artifacts/linux-build/build.log
cargo test --locked --release --no-default-features --features eos 2>&1 | tee artifacts/linux-build/tests.log
for binary in tsb tsb-server tsb-eos-check tsb-eos-peer-check libEOSSDK-Linux-Shipping.so; do
    cp "${CARGO_TARGET_DIR}/release/${binary}" target/linux-x64/release/
done
target/linux-x64/release/tsb-eos-check --offline | tee artifacts/linux-build/eos-offline.log
target/linux-x64/release/tsb --help > artifacts/linux-build/help.txt
ldd target/linux-x64/release/tsb | tee artifacts/linux-build/linked-libraries.txt
if grep -q 'not found' artifacts/linux-build/linked-libraries.txt; then
    exit 1
fi
