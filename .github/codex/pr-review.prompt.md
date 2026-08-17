# Codex output format

- Read the review context file whose absolute path is given at the end of these instructions; it holds the PR metadata and the diff commands.
- Return a markdown PR comment starting with `## Codex Review`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.
