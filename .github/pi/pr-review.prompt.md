# Pi output format

- Read the review context file whose absolute path is given at the end of these instructions; it holds the PR metadata and the diff (or the git commands to produce it).
- Return a markdown PR comment starting with `## Pi Review`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.
- Output ONLY the final review markdown — no preamble, no thinking, no tool transcripts.
