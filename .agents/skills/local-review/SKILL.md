---
name: local-review
description: Code review the current PR (or branch diff against main) for bugs, security, and AGENTS.md compliance. MUST use when asked to review code.
---

# Local Code Review

Run the same review locally that the GitHub auto-review actions run on PRs (Claude / Codex / Pi). The review policy lives in `REVIEW.md`.

**Why a subagent**: the review MUST run in a fresh context — not inline in the current session. If the user has been iterating on the diff, the main session has absorbed their reasoning and rationalizations, so it anchors and misses things CI catches. A subagent starts cold, like CI does.

## Steps

1. **Determine the PR scope** (cheap, do this in the main session):
   - If an argument is provided, treat it as a PR number or branch.
   - Otherwise, detect from the current branch vs `main`.
   - Confirm the PR/branch exists (`gh pr view <n>` or `git rev-parse <branch>`).

2. **Delegate the review to a fresh-context subagent** with a self-contained prompt. The prompt MUST include:
   - The PR number or branch name to review.
   - The instruction to read `REVIEW.md` first for the policy, then `AGENTS.md` files in directories touched by the diff.
   - The exact output format (see below).
   - Whether `--comment` was requested (so the subagent emits inline-comment payloads if needed).
   - Any "Additional reviewer instructions" the user provided.

   - **Claude Code**: use the `Agent` tool with `subagent_type: branch-diff-reviewer` (read-only tools, purpose-built for this). If unavailable, fall back to `general-purpose`.
   - **Codex / Pi**: if the CLI exposes a fresh-session subagent mechanism, use it. Otherwise tell the user to run the skill in a fresh CLI session and stop — running inline in the current session defeats the purpose.

3. **Receive the findings** from the subagent and relay them to the user verbatim. Do not re-summarize, re-judge, or filter — the whole point of fresh context is to surface what the main session would dismiss.

4. **Post comments if `--comment` was requested**: use the `gh` commands below with the subagent's output as the body. The main session does the posting because the subagent is read-only.

## Subagent prompt template

```
Review <PR #N | branch X> against main per the policy in REVIEW.md.

Steps:
1. Read REVIEW.md (repo root) for the full policy: severity triage, public-surface
   checklist, AGENTS.md compliance, test coverage assessment.
2. Read AGENTS.md (repo root) and any AGENTS.md in directories touched by the diff.
3. Get the diff: `gh pr diff <N>` (if PR) or `git diff main...<branch>`.
4. Get context: `gh pr view <N>` (if PR) or `git log main..<branch> --oneline`.
5. Read changed files only when the diff alone is insufficient to validate a finding.
6. Self-validate each finding: "is this definitely a real issue a senior engineer
   would flag?" Discard if uncertain.
7. Output findings in the exact format below. Do not modify any files.

<paste output format from below>

<if --comment requested:>
Additionally emit a JSON array of inline comments suitable for the GitHub reviews
API, one per finding that maps to a specific line:
[{"path": "...", "line": N, "side": "RIGHT", "body": "[P1] ..."}, ...]
```

## Output format

```
## Code review

<verdict line per REVIEW.md>

Found N issues:

1. [P0|P1|P2] <description>
   <file_path:line_number>

2. [P0|P1|P2] <description>
   <file_path:line_number>
```

End with a `Test coverage` section per the shared policy.

If no issues are found:

```
## Code review

Good to merge.

No issues found. Checked for bugs, security, and AGENTS.md compliance.
```

## Posting comments (`--comment`)

For a top-level PR comment:

```bash
gh pr review --comment --body "<summary from subagent>"
```

For inline comments on specific lines (using the JSON the subagent emitted):

```bash
gh api repos/{owner}/{repo}/pulls/{pr}/reviews \
  -f body="<summary>" -f event="COMMENT" -f comments="<json from subagent>"
```
