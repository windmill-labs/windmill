# Pi output format

- Read `./.github/pi/pr-review-context.md` for PR metadata and the diff commands.
- Return a markdown PR comment starting with `## Pi Review`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.
- Output ONLY the final review markdown — no preamble, no thinking, no tool transcripts.
