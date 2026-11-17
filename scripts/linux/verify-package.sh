#!/usr/bin/env bash
set -euo pipefail
package=${1:-/workspace/release/TSB-Linux-x64.tar.gz}
results=/workspace/artifacts/linux-build/package-check
mkdir -p "$results"
work=$(mktemp -d /tmp/tsb-package-check.XXXXXX)
tar -xzf "$package" -C "$work"
cd "$work"/TSB-Linux-x64
# Preserve captures and logs even when the software renderer exceeds a time limit.
collect_artifacts() {
    if [ -d artifacts ]; then
        mkdir -p "$results/runtime-artifacts"
        cp -a artifacts/. "$results/runtime-artifacts/"
    fi
}
trap collect_artifacts EXIT
test -x Play.sh
test -x tsb
readelf -h tsb > "$results/elf-header.txt"
readelf -d tsb > "$results/dynamic-section.txt"
ldd tsb > "$results/linked-libraries.txt"
if grep -q 'not found' "$results/linked-libraries.txt"; then
    cat "$results/linked-libraries.txt"
    exit 1
fi
./Play.sh --help > "$results/help.txt"
./tsb-eos-check --offline | tee "$results/eos-offline.log"

# Software OpenGL and a null playback device make graphics QA possible in Docker.
# Real GPU performance, audio hardware and separate-PC multiplayer need user testing.
export LIBGL_ALWAYS_SOFTWARE=1
export XDG_RUNTIME_DIR="$work/runtime"
mkdir -m 700 "$XDG_RUNTIME_DIR"
export ALSA_CONFIG_PATH="$work/asound.conf"
printf 'pcm.!default { type null }\n' > "$ALSA_CONFIG_PATH"
# The fixed 14-second gameplay smoke test cannot include llvmpipe's cold shader
# compilation. First exercise the frame-based startup review and warm Mesa's cache.
timeout 240s xvfb-run -a -s '-screen 0 1280x720x24' ./Play.sh --startup-review --home-review-small > "$results/startup.log" 2>&1
grep -q 'STARTUP_REVIEW_OK:' "$results/startup.log"
set +e
timeout 240s xvfb-run -a -s '-screen 0 1280x720x24' ./Play.sh --solo --smoke-test --hud-review-small > "$results/smoke.log" 2>&1
status=$?
set -e
tail -n 25 "$results/smoke.log"
if [ "$status" -ne 0 ]; then
    exit "$status"
fi
grep -q 'SMOKE_OK:' "$results/smoke.log"
echo 'PASS: extracted Linux package loaded the EOS SDK and rendered the game.'
