---
name: refine
user_invocable: true
description: End-of-session reflection. Reviews friction encountered during the session and proposes updates to docs/ to capture lessons learned.
---

# Refine Skill

Reflect on the current session and update documentation with lessons learned.

## Instructions

1. **Identify friction**: Review what happened in this session:
   - Run `git diff main...HEAD --stat` to see what files were touched
   - Think about: what was slow, what failed, what required multiple attempts, what information was missing or hard to find

2. **Read current docs**: Read the docs that were relevant to this session:
   - `docs/validation.md`
   - `docs/enterprise.md`
   - `docs/autonomous-mode.md`
   - Any skills that were invoked

3. **Propose updates**: For each piece of friction, decide if it warrants a doc update:
   - **Missing knowledge**: Information you had to discover that should be documented
   - **Wrong guidance**: Instructions that led you astray
   - **Missing validation rule**: A check that should be in the validation matrix
   - **New pattern**: A codebase pattern worth capturing for next time

4. **Apply updates**: Edit the relevant `docs/` files. Keep changes minimal and specific — add only what would have saved time this session.

5. **Report**: Summarize what was added/changed and why.

## Rules

- Only add knowledge confirmed by this session — no speculative additions
- Keep docs concise — add a line or two, not a paragraph
- If a whole new doc is needed, create it in `docs/` and add a pointer in `CLAUDE.md`
- Don't update skills unless a coding pattern was genuinely wrong
- Don't add things Claude already knows — only Windmill-specific knowledge
