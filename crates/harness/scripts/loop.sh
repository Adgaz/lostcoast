#!/usr/bin/env bash
# stage gate orchestrator. exits non-zero with a tagged STAGE_FAIL on first failure.
# tagged exits let the /loop wrapper decide whether to iterate or halt.
#
# usage: loop.sh <stage_id>

set -u
set -o pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <stage_id>" >&2
  exit 64
fi

stage_id="$1"
repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$repo_root"

export PATH="$HOME/.cargo/bin:$PATH"

cfg() { cargo run -q -p harness --bin stages -- get "$stage_id" "$1"; }

echo "==> stage $stage_id: fmt"
if ! cargo fmt --all -- --check; then
  echo "STAGE_FAIL: fmt" >&2
  exit 10
fi

echo "==> stage $stage_id: clippy"
if ! cargo clippy --workspace --all-targets -- -D warnings; then
  echo "STAGE_FAIL: clippy" >&2
  exit 11
fi

echo "==> stage $stage_id: build"
if ! cargo build --workspace; then
  echo "STAGE_FAIL: build" >&2
  exit 12
fi

# load config after build so the stages binary exists
bin="$(cfg bin)"
scene="$(cfg scene)"
camera="$(cfg camera)"
look_at="$(cfg look_at)"
size="$(cfg size)"
reference="$(cfg reference)"
visual_gate="$(cfg visual_gate)"
human_review="$(cfg human_review_required)"
ssim_min="$(cfg ssim_min)"
max_delta="$(cfg max_delta)"
tests="$(cfg numerical_tests)"

echo "==> stage $stage_id: tests ($tests)"
if [ -n "$tests" ]; then
  # un-ignore stage gate tests for this run only, by passing --include-ignored
  if ! cargo test --workspace -- --include-ignored $tests; then
    echo "STAGE_FAIL: tests" >&2
    exit 13
  fi
else
  if ! cargo test --workspace; then
    echo "STAGE_FAIL: tests" >&2
    exit 13
  fi
fi

if [ "$visual_gate" = "true" ]; then
  if [ -z "$scene" ] || [ -z "$camera" ]; then
    echo "STAGE_FAIL: stage config missing scene/camera" >&2
    exit 64
  fi

  mkdir -p crates/harness/captures
  actual="crates/harness/captures/stage-${stage_id}.actual.png"

  echo "==> stage $stage_id: render_ref under validation"
  set +e
  "$repo_root/crates/harness/scripts/run_with_validation.sh" "$stage_id" \
        "$repo_root/target/debug/render_ref" \
        --scene "$scene" \
        --camera "$camera" \
        --look-at "$look_at" \
        --size "$size" \
        --out "$actual"
  rc=$?
  set -e
  if [ "$rc" -ne 0 ]; then
    [ "$rc" -eq 70 ] && { echo "STAGE_FAIL: validation" >&2; exit 14; }
    [ "$rc" -eq 65 ] && exit 65
    echo "STAGE_FAIL: render_ref returned $rc" >&2
    exit 15
  fi

  if [ ! -f "$reference" ]; then
    echo "STAGE_FAIL: needs_reference (run approve_reference.sh $stage_id)" >&2
    exit 16
  fi

  echo "==> stage $stage_id: diff vs $reference"
  set +e
  cargo run -q -p harness --bin diff -- \
        --reference "$reference" \
        --actual "$actual" \
        --ssim-min "$ssim_min" \
        --max-delta "$max_delta"
  rc=$?
  set -e
  if [ "$rc" -ne 0 ]; then
    [ "$rc" -eq 1 ] && exit 17   # STAGE_FAIL: ssim
    [ "$rc" -eq 2 ] && exit 18   # STAGE_FAIL: delta
    [ "$rc" -eq 3 ] && exit 19   # STAGE_FAIL: size
    [ "$rc" -eq 4 ] && exit 20   # STAGE_FAIL: ref missing
    exit 21
  fi
fi

if [ "$human_review" = "true" ]; then
  echo "STAGE_PAUSE: human_review_required for stage $stage_id"
  exit 0
fi

echo "stage $stage_id PASS"
exit 0
