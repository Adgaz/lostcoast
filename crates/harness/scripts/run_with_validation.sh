#!/usr/bin/env bash
# run a target binary with vulkan validation enabled.
# tees stdout+stderr to a log, then greps for vuid/validation errors.
# exits non-zero if any are found.
#
# usage: run_with_validation.sh <stage_id> <bin_path> [bin_args...]

set -u
set -o pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <stage_id> <bin_path> [args...]" >&2
  exit 64
fi

stage_id="$1"
bin="$2"
shift 2

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
log_dir="$repo_root/crates/harness/captures"
mkdir -p "$log_dir"
log="$log_dir/stage-${stage_id}.validation.log"

# resolve vulkan sdk. lunarg installer puts it at ~/VulkanSDK/<ver>.
if [ -z "${VULKAN_SDK:-}" ]; then
  latest_sdk="$(ls -1d "$HOME/VulkanSDK"/*/macOS 2>/dev/null | sort | tail -n 1)"
  if [ -n "$latest_sdk" ]; then
    export VULKAN_SDK="$latest_sdk"
  fi
fi

if [ -z "${VULKAN_SDK:-}" ] || [ ! -d "$VULKAN_SDK" ]; then
  echo "STAGE_FAIL: vulkan sdk not found (set VULKAN_SDK or install lunarg sdk)" >&2
  exit 65
fi

export PATH="$HOME/.cargo/bin:$VULKAN_SDK/bin:$PATH"
export VK_LOADER_DEBUG="${VK_LOADER_DEBUG:-warn,error}"
export VK_INSTANCE_LAYERS="VK_LAYER_KHRONOS_validation"
export VK_LAYER_PATH="$VULKAN_SDK/share/vulkan/explicit_layer.d"
export VK_ICD_FILENAMES="$VULKAN_SDK/share/vulkan/icd.d/MoltenVK_icd.json"
export DYLD_LIBRARY_PATH="$VULKAN_SDK/lib:${DYLD_LIBRARY_PATH:-}"

set +e
"$bin" "$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
set -e

if grep -E "VUID-|Validation Error|VALIDATION" "$log" > /dev/null; then
  echo "STAGE_FAIL: validation messages in $log" >&2
  grep -E "VUID-|Validation Error|VALIDATION" "$log" | head -10 >&2
  exit 70
fi

exit "$status"
