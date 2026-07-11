---
name: local-review-codex
description: Run the CI Codex PR review locally against this branch's unpushed work (committed + uncommitted) before pushing. Same policy, model, and reasoning effort as the codex-pr-review GitHub action.
---

# Local Codex Review (pre-push)

Runs the exact same review Codex performs in CI (`.github/workflows/codex-pr-review.yml`),
but locally and scoped to work you have not pushed yet — so you catch what CI would flag
before the PR exists. Use this before `git push` on a non-trivial change.

**Correspondence with CI** — identical:
- Policy: `REVIEW.md` (severity triage, public-surface checklist, AGENTS.md compliance, test coverage).
- Model: `gpt-5.6-sol`, `model_reasoning_effort="xhigh"`.
- Output: markdown starting with `## Codex Review`, findings tagged P0 / P1 / P2 with file:line.

**Differences from CI** — local-only:
- Scope is the current branch vs `main` at the merge-base, **including uncommitted changes** (CI reviews a pushed PR diff).
- Sandbox is `read-only` (CI uses `danger-full-access` on an ephemeral runner). Codex reads the diff and files but cannot modify your working tree.
- Fresh context is inherent: `codex exec` is a separate cold process, so it does not anchor on the current chat session — the same reason `local-review` insists on a subagent.

## Prerequisites

- `codex` CLI **>= 0.144.1** installed and authed (`codex login` or `OPENAI_API_KEY`). Older CLIs reject `gpt-5.6-sol` with "requires a newer version of Codex". Upgrade with `npm install --global @openai/codex@0.144.1` (may need `sudo` for a global install). Keep this in sync with the pin in `.github/workflows/codex-pr-review.yml`.
- `git fetch` the base ref if it's stale, so the merge-base is accurate.

## Run

```bash
sh .agents/skills/local-review-codex/run.sh          # review vs main (default)
sh .agents/skills/local-review-codex/run.sh <base>   # review vs a different base ref
```

The script computes `BASE_SHA = git merge-base HEAD <base>`, feeds Codex `REVIEW.md` plus a
diff context pointing at `git diff <BASE_SHA>` (which folds in uncommitted edits), and prints
the review. It writes only temp files — nothing lands in the working tree.

## Relaying the result

Print the Codex output verbatim. Do not re-summarize or filter it — the value of a cold Codex
pass is surfacing what the current session would rationalize away. Then decide with the user
whether to address findings before pushing.

For a Claude-native review instead, use `local-review` (branch-diff-reviewer subagent). This
skill is the Codex counterpart; run both for independent perspectives.
