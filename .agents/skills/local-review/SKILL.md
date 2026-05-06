---
name: local-review
description: Code review the current PR (or branch diff against main) for bugs, security, and AGENTS.md compliance. MUST use when asked to review code.
---

# Local Code Review

Run the same review locally that the GitHub auto-review actions run on PRs (Claude / Codex / Pi). The review policy lives in `REVIEW.md` — read that first.

## Steps

1. **Read `REVIEW.md`** for the review policy: severity triage (P0 / P1 / P2), the new-public-surface checklist, AGENTS.md compliance, and the test-coverage assessment.

2. **Determine the PR scope**:
   - If an argument is provided, treat it as a PR number or branch.
   - Otherwise, detect from the current branch vs `main`.
   - Run `gh pr view` if a PR exists; otherwise compare against `main` with `git diff main...HEAD`.

3. **Get the diff and metadata**:
   - `gh pr diff` or `git diff main...HEAD` for the full diff.
   - `gh pr view` or `git log main..HEAD --oneline` for context.

4. **Read changed files** when the diff alone is insufficient.

5. **Apply the policy** from `REVIEW.md`. Self-validate each finding before reporting (real issue? would a senior engineer flag it?).

6. **Output findings** to the terminal (default) or post as PR comments (with `--comment` flag).

## Output format

```
## Code review

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

No issues found. Checked for bugs, security, and AGENTS.md compliance.
```

## Posting comments (`--comment`)

For a top-level PR comment:

```bash
gh pr review --comment --body "<summary>"
```

For inline comments on specific lines:

```bash
gh api repos/{owner}/{repo}/pulls/{pr}/reviews \
  -f body="<summary>" -f event="COMMENT" -f comments="[...]"
```
