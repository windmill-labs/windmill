# AI Evals

Black-box benchmark cases for the Windmill AI generation modes (`flow`, `app`,
`script`, `cli`, `global`).

**Authoring and running cases is documented in the `ai-evals` skill** — load it
before adding/changing a case or running a benchmark. Claude Code reads
`.claude/skills/ai-evals/SKILL.md`; Codex and Pi read
`.agents/skills/ai-evals/SKILL.md` (same canonical file). Invoke with `/ai-evals` in
Claude Code, `$ai-evals` in Codex, or `pi --skill ai-evals`.

For AI chat / copilot changes that these evals measure, see the `ai-chat` skill.

The full case format, fields, and fixture details remain in `ai_evals/README.md`.
