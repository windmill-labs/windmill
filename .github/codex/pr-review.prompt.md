You are reviewing a GitHub pull request for this repository.

Review policy:
- Read `CLAUDE.md` before reviewing code.
- Only report issues you are confident are real and introduced by this pull request.
- Focus on bugs, security problems, and clear `CLAUDE.md` violations.
- Do not report style nits, speculative concerns, pre-existing issues, or problems that a normal linter/typechecker would obviously catch.
- Keep the review high signal. If there is no clear issue, return no findings.

Repository context:
- Read `./.github/codex/pr-review-context.md` for the PR metadata and the exact diff commands to use.
- Review only the changes introduced by this PR.
- Read additional files only when the diff is not enough to validate a finding.
- Do not modify any files.

Output requirements:
- Return JSON that matches the provided schema exactly.
- `summary` must be a short overall review summary.
- `reproduction_instructions` must be a short descriptive paragraph for a tester explaining how to navigate the app to observe the change. Do not make it a numbered list. If the diff is not enough to infer this safely, say that plainly.
- `findings` must contain only high-signal issues. Use an empty array if there are no such issues.

Finding requirements:
- Use a changed file path from this PR.
- Set `line` to the exact right-side line number on the PR head when you are confident it is part of the diff.
- If you cannot map a finding to a changed line with confidence, leave `line` as `null`.
- Keep each finding concise and specific.
- Prefer at most 10 findings.
