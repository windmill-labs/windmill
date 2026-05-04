---
name: local-review
user_invocable: true
description: Code review a pull request for bugs and CLAUDE.md compliance. MUST use when asked to review code.
---

# Local Code Review Skill

Run the same review locally that the GitHub Claude Auto Review action runs on PRs. The review policy lives in `.github/review-prompt-shared.md` (severity triage, public-surface checklist, `AGENTS.md` compliance, test-coverage assessment); `.claude/review-prompt.md` holds Claude-specific output preferences. Read both before reviewing.

## Execution Steps

1. **Read `.github/review-prompt-shared.md`** for the review policy and `.claude/review-prompt.md` for the Claude output format

2. **Determine the PR scope**:
   - If an argument is provided, use it as the PR number or branch
   - Otherwise, detect from the current branch vs main
   - Run `gh pr view` if a PR exists, or use `git diff main...HEAD`

3. **Get the diff and metadata**:
   - `gh pr diff` or `git diff main...HEAD` for the full diff
   - `gh pr view` or `git log main..HEAD --oneline` for context

4. **Read changed files** where the diff alone is insufficient to understand context

5. **Apply the review policy from `.github/review-prompt-shared.md`** (and the output format from `.claude/review-prompt.md`)

6. **Self-validate each finding**: Before reporting, ask yourself:
   - "Is this definitely a real issue, not a false positive?"
   - "Would a senior engineer flag this in review?"
   - If the answer to either is no, discard the finding

7. **Output findings** to the terminal (default) or post as PR comments (with `--comment` flag)

## Output Format

```
## Code review

Found N issues:

1. [P0|P1|P2] <description>
   <file_path:line_number>

2. [P0|P1|P2] <description>
   <file_path:line_number>
```

End with a Test coverage section per `.github/review-prompt-shared.md`.

If no issues are found:

```
## Code review

No issues found. Checked for bugs, security, and AGENTS.md compliance.
```

## Posting Comments (--comment flag)

If the user passes `--comment`, post findings as inline PR comments using:

```bash
gh pr review --comment --body "<summary>"
```

Or for inline comments on specific lines:

```bash
gh api repos/{owner}/{repo}/pulls/{pr}/reviews -f body="<summary>" -f event="COMMENT" -f comments="[...]"
```
