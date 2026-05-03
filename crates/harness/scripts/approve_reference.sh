#!/usr/bin/env bash
# render the current state for a stage, open the result in preview,
# wait for the user to accept, then promote it to the committed reference.
#
# usage: approve_reference.sh <stage_id>

set -eu
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

scene="$(cfg scene)"
camera="$(cfg camera)"
look_at="$(cfg look_at)"
size="$(cfg size)"
reference="$(cfg reference)"
visual_gate="$(cfg visual_gate)"

if [ "$visual_gate" != "true" ]; then
  echo "stage $stage_id has visual_gate=false; nothing to approve" >&2
  exit 0
fi

mkdir -p "crates/harness/captures" "$(dirname "$reference")"
tmp="crates/harness/captures/stage-${stage_id}.tmp.png"

cargo run -q -p harness --bin render_ref -- \
  --scene "$scene" \
  --camera "$camera" \
  --look-at "$look_at" \
  --size "$size" \
  --out "$tmp"

open "$tmp"
printf "accept %s as reference for stage %s? [y/N] " "$tmp" "$stage_id"
read -r ans
case "$ans" in
  y|Y|yes|YES)
    mv "$tmp" "$reference"
    git add "$reference"
    echo "promoted $reference"
    ;;
  *)
    echo "rejected; tmp left at $tmp"
    exit 1
    ;;
esac
