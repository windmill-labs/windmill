---
name: local-review
user_invocable: true
description: Code review a pull request for bugs and CLAUDE.md compliance. MUST use when asked to review code.
---

# Local Code Review Skill

Review a pull request for real bugs and CLAUDE.md compliance violations. This review targets HIGH SIGNAL issues only.

## Review Philosophy

- **Only flag issues you are certain about.** If you are not sure an issue is real, do not flag it. False positives erode trust and waste reviewer time.
- Think like a senior engineer doing a final review — flag things that would cause incidents, not things that are merely imperfect.

## What to Flag

- Code that won't compile or parse (syntax errors, type errors, missing imports)
- Code that will definitely produce wrong results regardless of inputs
- Clear, unambiguous CLAUDE.md violations (quote the exact rule being violated)
- Security issues in introduced code (injection, auth bypass, data exposure)
- Incorrect logic that will fail in production

## What NOT to Flag

- Code style or quality concerns
- Potential issues that depend on specific inputs or runtime state
- Subjective suggestions or improvements
- Pre-existing issues not introduced by this PR
- Pedantic nitpicks a senior engineer wouldn't flag
- Issues a linter or type checker will catch
- General quality concerns unless explicitly prohibited in CLAUDE.md
- Issues silenced via lint ignore comments

## Execution Steps

1. **Determine the PR scope**:
   - If an argument is provided, use it as the PR number or branch
   - Otherwise, detect from the current branch vs main
   - Run `gh pr view` if a PR exists, or use `git diff main...HEAD`

2. **Find relevant CLAUDE.md files**:
   - Read the root `CLAUDE.md`
   - Check for CLAUDE.md files in directories containing changed files

3. **Get the diff and metadata**:
   - `gh pr diff` or `git diff main...HEAD` for the full diff
   - `gh pr view` or `git log main..HEAD --oneline` for context

4. **Read changed files** where the diff alone is insufficient to understand context

5. **Review for**:
   - CLAUDE.md compliance — check each rule against the changed code
   - Bugs and logic errors — will this code work correctly?
   - Security issues — injection, auth, data exposure in new code

6. **Self-validate each finding**: Before reporting, ask yourself:
   - "Is this definitely a real issue, not a false positive?"
   - "Would a senior engineer flag this in review?"
   - If the answer to either is no, discard the finding

7. **Output findings** to the terminal (default) or post as PR comments (with `--comment` flag)

## Output Format

```
## Code review

Found N issues:

1. <description> (<reason: CLAUDE.md adherence | bug | security>)
   <file_path:line_number>

2. <description> (<reason>)
   <file_path:line_number>
```

If no issues are found:

```
## Code review

No issues found. Checked for bugs and CLAUDE.md compliance.
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
