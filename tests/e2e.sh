#!/usr/bin/env bash
# End-to-end check of pir in --plain mode against a throwaway git repo:
# good commit -> restart, broken commit -> old tool keeps running,
# SIGINT -> exit 130 with no leftover processes (tool ignores SIGTERM on purpose).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -q
PIR=$PWD/target/debug/pir
T=$(mktemp -d)
trap 'kill -9 "${PID:-0}" 2>/dev/null || true; pkill -9 -f "$T/sleeper" || true; rm -rf "$T"' EXIT

cp /bin/sleep "$T/sleeper"
git init -q --bare -b trunk "$T/origin.git"
git clone -q "$T/origin.git" "$T/work" 2>/dev/null
commit() { echo "$1" > "$T/work/version"; git -C "$T/work" add version; git -C "$T/work" -c user.name=t -c user.email=t@t commit -qm "$1"; git -C "$T/work" push -q origin HEAD:trunk 2>/dev/null; }
commit v1
git clone -q "$T/origin.git" "$T/clone"

wait_for() { # pattern [count]
  for _ in $(seq 100); do [ "$(grep -c -- "$1" "$T/out.log" || true)" -ge "${2:-1}" ] && return; sleep 0.2; done
  echo "FAIL: timed out waiting for '$1' x${2:-1}"; cat "$T/out.log"; exit 1
}

"$PIR" -C "$T/clone" --plain --every 1s --stop-timeout 1s --log-dir "$T/logs" \
  --build "if grep -q broken version; then echo 'compile error'; exit 1; fi; cp version $T/installed" \
  --run "trap '' TERM; $T/sleeper 1000 & echo running \$(cat $T/installed); while :; do sleep 0.2; done" \
  > "$T/out.log" 2>&1 &
PID=$!

wait_for "running v1"
commit v2;     wait_for "running v2"
commit broken; wait_for "build FAILED"
sleep 1.5
[ "$(grep -c 'run started' "$T/out.log")" -eq 2 ] || { echo "FAIL: broken build restarted the tool"; cat "$T/out.log"; exit 1; }
commit v3;     wait_for "running v3"
[ "$(grep -c '── build #' "$T/out.log")" -eq 4 ] || { echo "FAIL: expected 4 builds"; cat "$T/out.log"; exit 1; }

kill -INT $PID
set +e; wait $PID; code=$?; set -e
[ $code -eq 130 ] || { echo "FAIL: exit code $code, want 130"; cat "$T/out.log"; exit 1; }
if pgrep -f "$T/sleeper" >/dev/null; then echo "FAIL: leftover processes"; pgrep -fl "$T/sleeper"; exit 1; fi
ls "$T"/logs/run-*.log >/dev/null && grep -q "running v3" "$T"/logs/run-*.log || { echo "FAIL: run logs missing"; exit 1; }
ls "$T"/logs/build-*.log | wc -l | grep -q 4 || { echo "FAIL: want 4 build logs"; exit 1; }
echo "PASS"
