#!/usr/bin/env bash
set -euo pipefail

HOURS=12
CASES=20000
MAX_SHRINK=5000
THREADS=1

usage() {
  cat <<EOF
Usage: ./scripts/proptest-hammer.sh [options]

Options:
  --hours=N    Duration in hours (default: 12)
  --cases=N    Proptest cases per run (default: 20000)
  --shrink=N   Max shrink iterations (default: 5000)
  --threads=N  Test threads (default: 1)
  --help       Show this message

Env overrides also work: SUFFETE_PROPTEST_CASES, SUFFETE_PROPTEST_MAX_SHRINK_ITERS.
EOF
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --hours=*)   HOURS="${1#*=}"; shift ;;
    --cases=*)   CASES="${1#*=}"; shift ;;
    --shrink=*)  MAX_SHRINK="${1#*=}"; shift ;;
    --threads=*) THREADS="${1#*=}"; shift ;;
    --help)      usage ;;
    *) echo "unknown option: $1"; usage ;;
  esac
done

# env vars take precedence over flags
CASES="${SUFFETE_PROPTEST_CASES:-$CASES}"
MAX_SHRINK="${SUFFETE_PROPTEST_MAX_SHRINK_ITERS:-$MAX_SHRINK}"
MAX_REJECTS=$((CASES * 4))

OUTDIR="target/proptest-hammer"
mkdir -p "$OUTDIR"
rm -rf "$OUTDIR"/*
rm -f tests/property_lattice.proptest-regressions

START_TS=$(date +%s)
END_TS=$((START_TS + HOURS * 3600))
RUN=0

echo "=== proptest-hammer ==="
echo "  hours     : $HOURS"
echo "  cases/run : $CASES"
echo "  shrink    : $MAX_SHRINK"
echo "  threads   : $THREADS"
echo "  rejects   : $MAX_REJECTS"
echo "  logs      : $OUTDIR"
echo "  start     : $(date)"
echo

while :; do
  RUN=$((RUN + 1))
  NOW=$(date +%s)
  ELAPSED=$(((NOW - START_TS) / 60))
  REMAINING=$(((END_TS - NOW) / 60))
  [ "$REMAINING" -lt 0 ] && REMAINING=0
  echo -n "[run $RUN] +${ELAPSED}m -${REMAINING}m ... "

  LOG="$OUTDIR/run-${RUN}.log"
  if SUFFETE_PROPTEST_CASES="$CASES" \
     SUFFETE_PROPTEST_MAX_SHRINK_ITERS="$MAX_SHRINK" \
     SUFFETE_PROPTEST_MAX_GLOBAL_REJECTS="$MAX_REJECTS" \
     cargo t --test property_lattice -- --test-threads="$THREADS" --nocapture \
     > "$LOG" 2>&1; then
    echo "ok"
    rm "$LOG"
  else
    FAILURES=$(grep -c "panicked at" "$LOG" || true)
    echo "${FAILURES} failure(s)"
  fi

  rm -f tests/property_lattice.proptest-regressions

  NOW=$(date +%s)
  [ "$NOW" -lt "$END_TS" ] || break
done

echo
echo "=== finished: $(date) ==="
echo

declare -A seen=()
for log in "$OUTDIR"/run-*.log; do
  [ -f "$log" ] || continue
  while IFS= read -r line; do
    test_name="${line#test }"
    test_name="${test_name%% *}"
    if [ -n "$test_name" ] && [ -z "${seen[$test_name]:-}" ]; then
      seen[$test_name]="$log"
    fi
  done < <(grep "panicked at" "$log" | sed -n 's/^test \([^ ]*\) .*/\1/p' || true)
done

if [ ${#seen[@]} -eq 0 ]; then
  echo "All clean — no failures across $RUN runs."
  exit 0
fi

echo "Unique failing tests (${#seen[@]}):"
echo "================================"
for test_name in "${!seen[@]}"; do
  log="${seen[$test_name]}"
  echo
  echo "--- $test_name ---"
  awk '/Test failed:/{found=1} found{print} /^    \) *$/{if(found) exit}' "$log"
done
