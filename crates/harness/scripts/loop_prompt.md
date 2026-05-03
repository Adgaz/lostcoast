# /loop prompt template

Paste the text below into `/loop` to drive a stage to green. Replace `<N>` with the stage id.

```
You are iterating on stage <N> of lostcoast. Authoritative refs:
- docs/stages/stage-<NN>.md
- docs/algorithm-notes.md (math; if code disagrees, code is wrong)
- docs/testing.md (gate recipes)
- crates/harness/stages.toml (the gate config for this stage)
- crates/harness/README.md (what the loop does and does not check)

Each iteration:
1. Read the stage doc and the loop's last failure tag.
2. Implement the smallest change that addresses the failure. Edit existing files; do not create new ones unless the stage doc requires it.
3. Run: bash crates/harness/scripts/loop.sh <N>
4. Inspect the exit. Tags:
   - STAGE_FAIL: fmt | clippy | build | tests | validation | ssim | delta | size — fix and re-run
   - STAGE_FAIL: needs_reference — halt and tell the user to run approve_reference.sh <N>
   - STAGE_PAUSE: human_review_required — halt and tell the user to eyeball
   - exit 0 stage PASS — git add the changed files, git commit, halt
5. Commits: lowercase subject, ≤60 chars, no Co-Authored-By, no Claude footer, no emojis (CLAUDE.md).

Hard caps for this loop call:
- Max 10 iterations. Halt regardless and report.
- Do not edit docs/ or CLAUDE.md.
- Do not edit other stages' code.
- Do not skip the closed-box energy test (stage 7) or the half-Lambert table (stage 5).
- If the same tag fires 3 iterations in a row with no diff in the failure, halt and ask.
```

## invoking

```sh
/loop crates/harness/scripts/loop_prompt.md
```

or with a self-paced interval omitted, let the agent decide cadence.

## stopping

Type `stop` in `/loop` or hit Ctrl-C. The agent will not commit a half-finished stage; the last successful commit is the rollback point.
