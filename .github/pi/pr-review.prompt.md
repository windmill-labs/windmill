# Pi output format

- Read the review context file whose absolute path is given at the end of these instructions; it holds the PR metadata and the diff (or the git commands to produce it).
- Return a markdown PR comment starting with `## Pi Review`.
- Tag each finding with a severity (P0 / P1 / P2), file path, and line number when known confidently.
- Output ONLY the final review markdown — no preamble, no thinking, no tool transcripts.

# Before you settle on a verdict

`REVIEW.md` tells you to discard findings you are not confident in. That rule exists to suppress noise, not to license a quick approval. Review in two passes:

1. Enumerate every candidate defect you notice, without judging any of them yet.
2. Take each candidate and try to prove it is real: read the surrounding code, check the caller, check the error path. Keep it, or dismiss it for a specific reason.

A "Good to merge" verdict must be accompanied by a "Considered and dismissed" section listing each candidate from pass 1 with the concrete reason it is not a finding. If that section would be empty, pass 1 was skipped: go back and do it.

Facts cut both ways. If you notice that a cached value can be multiple megabytes, that a lock is held across an await, or that a new parameter is caller-controlled, that observation is a candidate for pass 2 even when the surrounding code looks deliberate. Do not narrate such a fact as evidence that the code is fine without first checking whether it is a bug.
