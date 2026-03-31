# Code Review Instructions

Review this pull request and provide comprehensive feedback.

## Focus Areas

- **Code quality and best practices** — does the code follow established patterns?
- **Potential bugs or issues** — will this code work correctly in all cases?
- **Performance considerations** — are there unnecessary allocations, N+1 queries, or bottlenecks?
- **Security implications** — injection, auth bypass, data exposure?

## CLAUDE.md Compliance

Read all relevant CLAUDE.md files (root and in directories containing changed files). Check each rule against the changed code. Quote the exact rule when flagging a violation.

## Review Guidelines

- Provide detailed feedback using inline comments for specific issues
- Use top-level comments for general observations or praise
- Only flag issues introduced by this PR, not pre-existing problems
- Self-validate each finding: "Is this definitely a real issue?" If uncertain, discard it

## Testing Instructions

At the end of your review, add complete instructions to reproduce the added changes through the app interface. These instructions will be given to a tester so they can verify the changes. It should be a short descriptive text (not a step-by-step or a list) on how to navigate the app (what page, what action, what input, etc.) to see the changes.
