# Vendored skills

These five skills are copied from an external repository, not written here:

- `grill-me`, `grilling`
- `improve-codebase-architecture`, `codebase-design`, `domain-modeling`

Source: https://github.com/mattpocock/skills
Pinned at commit `84fdeffd12f2ee307994d1eb6feb48173b6e0502`.

They form one dependency closure — `grill-me` is a stub that runs `grilling`, and
`improve-codebase-architecture` draws its vocabulary from `codebase-design` and its
CONTEXT.md upkeep from `domain-modeling`. Removing any one breaks the others.

Local changes on top of upstream, kept to the minimum so a refresh stays a diff:

- Flattened the upstream `skills/engineering/` and `skills/productivity/` split, since this
  repo's skills are flat.
- Replaced each SKILL.md's markdown links to its own bundled files with plain repo-root paths
  in prose (`.agents/skills/<skill>/FILE.md`). Upstream's sibling-relative links break when the
  file is read through the `.claude/skills/<skill>/SKILL.md` symlink, which mirrors only
  SKILL.md — and a repo-root *link* is equally wrong, since a markdown target resolves relative
  to the file containing it. Companion files keep their sibling-relative links; they are only
  ever read at their real path, never through the symlink.
- Dropped the upstream `agents/openai.yaml` files — Codex packaging metadata for that repo's
  own plugin distribution, unused here.
- **Removed every ADR path.** Upstream, `domain-modeling` offers to write Architecture Decision
  Records into `docs/adr/` and `improve-codebase-architecture` reads and cites them. This repo has
  not adopted ADRs, and a skill that offers to create them is how the practice arrives by side
  effect rather than by decision. Deleted `domain-modeling/ADR-FORMAT.md`, its "Offer ADRs
  sparingly" section, and the `docs/adr/` entries in its file-structure diagrams; dropped the ADR
  clauses from `improve-codebase-architecture` (intro, explore step, "ADR conflicts", the
  offer-an-ADR bullet in the grilling loop) and the ADR callout row in `HTML-REPORT.md`. Also cut
  "record an architectural decision" from `domain-modeling`'s description, since that phrase is an
  invocation trigger. What remains is CONTEXT.md and ubiquitous-language work only.

To refresh, diff against the same paths at a newer commit and re-apply these four changes. The
ADR removal is the one that needs judgement: if the team later adopts ADRs, take upstream's
version of those sections back rather than rewriting them here.

## License

MIT License

Copyright (c) 2026 Matt Pocock

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
