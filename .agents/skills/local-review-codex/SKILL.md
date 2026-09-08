---
name: local-review-codex
description: Run the CI Codex PR review locally against this branch's unpushed work (committed + uncommitted) before pushing. Same policy and reasoning effort as the codex-pr-review GitHub action, on a newer model.
---

# Local Codex Review (pre-push)

Runs the exact same review Codex performs in CI (`.github/workflows/codex-pr-review.yml`),
but locally and scoped to work you have not pushed yet — so you catch what CI would flag
before the PR exists. Use this before `git push` on a non-trivial change.

**Correspondence with CI** — identical:
- Policy: `REVIEW.md` (severity triage, public-surface checklist, AGENTS.md compliance, test coverage).
- Reasoning effort: `model_reasoning_effort="xhigh"`.
- Output: markdown starting with `## Codex Review`, findings tagged P0 / P1 / P2 with file:line.

**Differences from CI** — local-only:
- Model is `gpt-6-astra`; CI stays on `gpt-5.6-sol`. Not an oversight to reconcile: `gpt-6-astra` is confirmed on the ChatGPT auth `codex login` uses locally, while CI authenticates with `OPENAI_API_KEY` (`codex-pr-review.yml` prefers it over `CODEX_AUTH_JSON`) and that tier is unverified for the model. Move CI once API access is confirmed, or once CI switches to `CODEX_AUTH_JSON`.
- Scope is the current branch vs `main` at the merge-base, **including uncommitted changes** (CI reviews a pushed PR diff).
- Sandbox is `read-only` (CI uses `danger-full-access` on an ephemeral runner). Codex reads the diff and files but cannot modify your working tree.
- Fresh context is inherent: `codex exec` is a separate cold process, so it does not anchor on the current chat session — the same reason `local-review` insists on a subagent.

## Prerequisites

- `codex` CLI **>= 0.153.4** installed and authed via `codex login` (an `OPENAI_API_KEY` in the environment takes priority and may not reach `gpt-6-astra` — see the model note above). Older CLIs reject the model with "requires a newer version of Codex"; `run.sh` checks the version up front. Upgrade with `npm install --global @openai/codex@0.153.4` (may need `sudo` for a global install). This matches the pin in `.github/workflows/codex-pr-review.yml` — the CLI version is the same on both sides, only the model differs.
- `git fetch` the base ref if it's stale, so the merge-base is accurate.

## Run

```bash
bash .agents/skills/local-review-codex/run.sh          # review vs main (default)
bash .agents/skills/local-review-codex/run.sh <base>   # review vs a different base ref
```

Invoke with `bash` (or run the executable directly) — the script needs Bash for
`set -o pipefail`; `sh` is Dash on Debian/Ubuntu and would fail. If `main` isn't a
local branch (e.g. a fresh single-branch checkout), the runner falls back to
`origin/main` automatically.

The script computes `BASE_SHA = git merge-base HEAD <base>`, feeds Codex `REVIEW.md` plus a
diff context pointing at `git diff <BASE_SHA>` (which folds in uncommitted edits), and prints
the review. It writes only temp files — nothing lands in the working tree.

## Relaying the result

Print the Codex output verbatim. Do not re-summarize or filter it — the value of a cold Codex
pass is surfacing what the current session would rationalize away. Then decide with the user
whether to address findings before pushing.

For a Claude-native review instead, use `local-review` (branch-diff-reviewer subagent). This
skill is the Codex counterpart; run both for independent perspectives.
