# harness

Per-stage gates for the autonomous build loop. The loop drives stages 1–12 from `docs/roadmap.md`. Each stage has a config block in `stages.toml` and runs through `scripts/loop.sh <id>`.

## what the loop checks

For every stage:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo build --workspace`
- `cargo test --workspace -- --include-ignored <numerical_tests>` (the per-stage filters)
- if `visual_gate = true`: render_ref under the vulkan validation layer, then diff the resulting PNG against `references/stage-NN.png` using SSIM + max ΔE
- if `human_review_required = true`: halt with `STAGE_PAUSE` after a green run

Tagged exit codes from `loop.sh`:

| code | tag | meaning |
|------|-----|---------|
| 10 | `fmt` | rustfmt complains |
| 11 | `clippy` | clippy warning at deny level |
| 12 | `build` | cargo build failed |
| 13 | `tests` | unit/integration test failed |
| 14 | `validation` | a `VUID-*` or validation error appeared in the layer log |
| 15 | `render_ref` | render_ref binary errored (e.g. scene load failed, gfx not wired) |
| 16 | `needs_reference` | first reference png does not exist; run `approve_reference.sh` |
| 17 | `ssim` | screenshot diverged below `ssim_min` |
| 18 | `delta` | per-pixel max ΔE above `max_delta` |
| 19 | `size` | reference and actual PNG dimensions differ |
| 20 | `ref_missing` | reference path set but file unreadable |
| 65 | n/a | vulkan SDK not on disk |

## what the loop does NOT check

These need your eyes. Do not let the loop sign off on them alone.

### renderdoc captures

The roadmap requires a RenderDoc capture per graphics stage. RenderDoc has no first-class macOS build and the loop never opens it. Procedure:

1. Run `app` (or `lightbaker`) attached to RenderDoc on a Linux/Windows box, or use the experimental macOS RenderDoc build.
2. Save the capture to `crates/harness/captures/stage-NN.rdc`.
3. Open it, label passes and resources, confirm nothing surprising.
4. Commit the `.rdc` only if it is small enough — the `.gitignore` excludes them by default; override per-file if you want one in history.

On macOS, Xcode's Metal frame debugger (Debug ▸ Capture GPU Frame) sees through MoltenVK and is the practical alternative.

### first reference screenshots

The loop cannot generate the first reference for a stage. By definition there is nothing to diff against. Workflow:

```sh
crates/harness/scripts/approve_reference.sh 3
```

Renders the current frame, opens the PNG in Preview, prompts for accept. On accept, moves the file into `references/` and `git add`s it. After that, every subsequent loop run regression-diffs against it.

### subtle look bugs (stages 6, 7, 8, 10, 11)

Stages flagged `human_review_required = true` in `stages.toml`. Loop runs the numerical gates but pauses for your eye. What to look for:

- **stage 6, direct lightmap** — shadow boundaries should be hard but not jagged; floor luxel directly under the light should match the analytic value from `docs/testing.md`.
- **stage 7, radiosity** — closed-box energy test passing is necessary, not sufficient. Color bleeding should look like radiosity, not a gaussian blur of the source. Spot-check that a red wall tints the adjacent white wall pinkish, not orange.
- **stage 8, RNM** — flat plane under overhead light: all three lightmaps must be visually identical. If one channel differs, basis vectors are signed wrong.
- **stage 10, HDR** — bloom must not bleed across hard edges; tonemap must not crush near-black. Step-change autoexposure should converge in ~30 frames per the test, but eyeball the curve too.
- **stage 11, parallax cubemaps** — sweep the camera and watch a ceiling-light reflection on a planar mirror. Without parallax it slides; with parallax it stays put. The numerical test only checks one camera.

### bug classes the loop misses

- energy leaks outside the closed-box scene (irregular geometry, large patches)
- RNM basis sign errors that happen to pass `flat_plane_three_equal`
- autoexposure overshoot that converges within 30 frames but oscillates
- bloom kernel asymmetry (numerical test only checks brightness, not symmetry)
- shader precision issues that show as banding in dark regions

## scripts

| script | purpose |
|--------|---------|
| `scripts/loop.sh <id>` | full gate pipeline for one stage |
| `scripts/run_with_validation.sh <id> <bin> [args...]` | run a binary under vulkan validation, fail on `VUID-*` |
| `scripts/approve_reference.sh <id>` | promote a fresh render to the committed reference |
| `scripts/loop_prompt.md` | text to paste into `/loop` so an iterating agent knows the rules |

## prerequisites

- rust 1.78+ via rustup
- cmake (shaderc crate vendors and builds glslc on first build)
- python3 (also for shaderc)
- vulkan SDK installed at `~/VulkanSDK/<ver>/macOS` — provides MoltenVK loader, validation layer, glslc binary
- (optional) RenderDoc on a Linux/Windows machine for graphics stage captures

## env

The validation runner exports the vulkan envs itself, sourced from `$VULKAN_SDK` or auto-detected from `~/VulkanSDK/`. You don't need to add anything to `.zshrc`. If you do want them in your interactive shell, source `$VULKAN_SDK/setup-env.sh`.
