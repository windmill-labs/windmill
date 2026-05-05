# Codex output format

- Read `./.github/codex/pr-review-context.md` for PR metadata and the diff commands.
- Return a markdown PR comment starting with `## Codex Review`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.
