# Output requirements (Codex)

## Repository context

- Read `./.github/codex/pr-review-context.md` for PR metadata and the exact diff commands to use. Run those `git log` / `git diff` commands to inspect the changes.

## Output format

- Return a GitHub PR comment in markdown, not JSON.
- Start with `## Codex Review`.
- Give a short overall summary first.
- List each finding with a severity tag (P0 / P1 / P2), file path, and line number when known confidently.
- If you found no high-signal issues, say that explicitly.
- End with a `### Reproduction instructions` section: a short descriptive paragraph for a tester explaining how to navigate the app to observe the change. Do not make it a numbered list. If the diff is not enough to infer this safely, say that plainly.
