---
name: pr
user_invocable: true
description: Open a draft pull request on GitHub and drive CI review rounds until it is ready. MUST use when you want to create/open a PR.
---

# Pull Request Skill

Create a draft pull request with a clear title and explicit description of changes, then drive it through CI review rounds to ready.

## Instructions

1. **Analyze branch changes**: Understand all commits since diverging from main
2. **Push to remote**: Ensure all commits are pushed
3. **Create draft PR**: Always open as draft for review before merging
4. **Drive review rounds**: trigger CI reviews on the draft and only flip to ready once every verdict is a go (see "Review rounds" below)

## PR Title Format

Follow conventional commit format for the PR title:
```
<type>: <description>
```

### Types
- `feat`: New feature or capability
- `fix`: Bug fix
- `refactor`: Code restructuring
- `docs`: Documentation changes
- `chore`: Maintenance tasks
- `perf`: Performance improvements

### Title Rules
- Keep under 70 characters
- Use lowercase, imperative mood
- No period at the end
- If `*_ee.rs` files were modified, prefix with `[ee]`: `[ee] <type>: <description>`

## PR Body Format

The body MUST be explicit about what changed. Structure:

```markdown
## Summary
<Clear description of what this PR does and why>

## Changes
- <Specific change 1>
- <Specific change 2>
- <Specific change 3>

## Test plan
- [ ] <How to verify change 1>
- [ ] <How to verify change 2>
```

The harness/tooling that invoked the skill may add its own attribution trailer; the skill itself does not prescribe one.

## Screenshots (required for frontend changes)

If `git diff main...HEAD --name-only` matches `^frontend/`, the PR body **must** include
screenshots of the affected UI. Skip only when there is no visible UI effect (types,
tests, build config) — and say so in the body.

1. Verify the change in the browser (AGENTS.md → "Verifying Frontend Changes").
2. Screenshot each affected page with `mcp__playwright__browser_take_screenshot` (save to a file).
3. Host each image and get its Markdown embed by pushing to the public
   `windmill-labs/agent-screenshots-internal` repo. **Pipe base64 through stdin** —
   passing it as `-f content=…` fails with `argument list too long` on real images:

   ```bash
   REPO=windmill-labs/agent-screenshots-internal
   IMG=screenshot.png                                          # repeat per page
   DEST="shots/$(git branch --show-current)/$(date +%s)-$(basename "$IMG")"
   base64 -w0 "$IMG" | jq -Rs --arg m "add $DEST" '{message:$m, content:.}' \
     | gh api -X PUT "repos/$REPO/contents/$DEST" --input - >/dev/null
   echo "![$(basename "$IMG" .png)](https://raw.githubusercontent.com/$REPO/main/$DEST)"
   ```
   Derive `$DEST` from the file name (as above) so distinct pages never collide — a
   fixed name would make same-second uploads reuse one path, and the second `PUT`
   then 422s (the Contents API needs the existing file's `sha` to overwrite).
4. Put the printed `![…](…)` lines under a `## Screenshots` heading in the PR body.

Requires `gh` (`repo` scope), `jq`, `base64` — all in the devShell. The host repo is
public (so the raw URLs render for reviewers without a token) and its history is
permanent — **never screenshot pages that show secrets or sensitive values** (workspace
variables, resource values, instance settings, OAuth/SMTP config); deleting the file
can't undo an accidental capture. (GitHub's drag-and-drop uploader needs a browser
session and can't be driven from a token.)

If `gh` can't push to the host repo (e.g. a CI token scoped only to `windmill`), do
**not** fail the PR or skip silently — hand the upload to the user, who has push access,
and continue once they confirm it's done.

## Execution Steps

1. Run `git status` to check for uncommitted changes
2. Run `git log main..HEAD --oneline` to see all commits in this branch
3. Run `git diff main...HEAD` to see the full diff against main
4. **Review the diff before creating the PR — run both reviews, do not skip:**
   - **`local-review`** — Claude-native branch-diff-reviewer (`/local-review` in Claude Code, `$local-review` in Codex, `pi --skill local-review` / `/skill:local-review` in Pi).
   - **`local-review-codex`** — cold Codex pass, the same review CI runs, for an independent perspective the Claude pass misses (`/local-review-codex` in Claude Code, or `bash .agents/skills/local-review-codex/run.sh`). If the `codex` CLI is missing or older than the version pinned in that skill, note it in your summary and continue — never block the PR on codex being unavailable.

   Run both — they catch different things. If either surfaces issues, fix them and commit before proceeding.
5. **Screenshots for frontend changes**: if `git diff main...HEAD --name-only` matches `^frontend/`, capture and embed screenshots of the affected UI per "Screenshots" above before writing the PR body (skip only if there is no visible UI effect).
6. Check if remote branch exists and is up to date:
   ```bash
   git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || echo "no upstream"
   ```
7. Push to remote if needed: `git push -u origin HEAD`
8. Create draft PR using gh CLI:
   ```bash
   gh pr create --draft --title "<type>: <description>" --body "$(cat <<'EOF'
   ## Summary
   <description>

   ## Changes
   - <change 1>
   - <change 2>

   ## Test plan
   - [ ] <test 1>
   - [ ] <test 2>
   EOF
   )"
   ```
9. Return the PR URL to the user
10. Drive the PR through CI review rounds to ready (see "Review rounds" below)

## Review rounds (draft → ready)

A PR leaves draft **only after a clean CI review round**. Never run `gh pr ready` before that.

1. **Trigger a round and wait for it**: launch the waiter as a background Bash task (a round takes 10–30 min; you are woken when it exits — do not stop the session or poll in the foreground while it runs):

   ```bash
   bash .agents/skills/pr/review-round.sh <PR_NUMBER>
   ```

   It comments `/review` on the PR — which runs the Codex, Claude and Pi CI reviewers even on a draft — waits for the spawned `PR Review Commands` workflow run(s) to complete, then prints one verdict line per reviewer and saves the full review comments to files.

2. **Judge the round.** Codex is mandatory; Claude, Pi and cubic count whenever they posted. Every review starts with one of the three `REVIEW.md` verdicts:
   - Codex verdict missing → the round is void: comment `/codex` on the PR, wait for it the same way, and judge again.
   - Any **"Should address issues before merging"** → fix the P0/P1 findings (and the nits while you're there), commit, push, and start a new round (step 1).
   - Only **"Mergeable, but should ideally address nits"** and/or **"Good to merge"** → fix the nits too; a nit that is wrong or genuinely not worth fixing may instead be dismissed by replying to the review comment with your reasoning. Push nit-only fixes without starting another full round.

3. **Flip to ready with the marker comment.** The review workflows skip the redundant `ready_for_review`-triggered round when the PR author has posted a marker naming the current head SHA **and** the PR's latest Codex review *posted before the marker* has a non-blocking verdict (reviewer evidence — a bare marker with no round behind it, or one whose last pre-marker Codex verdict is "Should address issues", skips nothing). Keep the prefix exact and use the full 40-char SHA of the head you are flipping:
   - every verdict was "Good to merge" (head unchanged since the round):

     `✅ Review round clean @ <head-sha>`

   - nit-only round, nits fixed or dismissed afterwards (head may have moved past the reviewed SHA — say so):

     `✅ Review round clean @ <head-sha> — nit-only verdicts at <round-sha>; nits addressed in <commit sha(s)> / dismissed in review replies`

   ```bash
   gh pr comment <PR_NUMBER> --body "✅ Review round clean @ $(git rev-parse HEAD)"
   gh pr ready <PR_NUMBER>
   ```

   If any P0/P1 finding is unaddressed or the head moved for reasons other than nit fixes, do **not** post the marker or flip — run another round instead.

## EE Companion PR (when `*_ee.rs` files were modified)

The `*_ee.rs` files in the windmill repo are **symlinks** to `windmill-ee-private` — changes won't appear in `git diff` of the windmill repo. Instead, check the EE repo for uncommitted or unpushed changes.

Follow the full EE PR workflow in `docs/enterprise.md`. The key PR-specific details:

1. Find the EE repo/worktree: see "Finding the EE Repo" in `docs/enterprise.md`
2. Check for changes: `git -C <ee-path> status --short`
   - If there are no changes in the EE repo, skip this entire section
3. Follow steps 1–5 from the "EE PR Workflow" in `docs/enterprise.md`
4. Create the companion PR (title does NOT get the `[ee]` prefix):
   ```bash
   gh pr create --draft --repo windmill-labs/windmill-ee-private --title "<type>: <description>" --body "$(cat <<'EOF'
   Companion PR for windmill-labs/windmill#<PR_NUMBER>
   EOF
   )"
   ```
5. Commit `ee-repo-ref.txt` and push the updated windmill branch
