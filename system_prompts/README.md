# System Prompts

The single source of the AI guidance Windmill ships: what an agent needs to know about Windmill
itself to write flows, scripts, apps, pipelines, resources, triggers and schedules. Two consumers
are built from it:

- **the chat** (the frontend AI chat: flow, script, app and global modes), through the
  `$system_prompts` alias to `auto-generated/`;
- **the CLI** (`wmill init` / `wmill refresh prompts`), as the skills embedded in
  `cli/src/guidance/skills.gen.ts`, next to the `AGENTS.wmill.md` template in
  `cli/src/guidance/core.ts`.

Guidance written in one consumer only is guidance the other never gets. Write it here.

## Structure

```
system_prompts/
├── base/              # Hand-written guidance, one file per topic (flow-base.md, raw-app.md, …)
│                      # plus CLI-only intros (flow-cli.md, script-cli.md, raw-app-cli.md, …)
├── languages/         # Hand-written per-language guidance (bun.md, python3.md, …)
├── generate.py        # Builds everything under auto-generated/ and cli/src/guidance/skills.gen.ts
├── utils.py           # Shared helpers, including the fence renderer
└── auto-generated/    # Generated — never edit
    ├── prompts.ts     # One export per base/ and languages/ file (chat render), SDK docs, schema
    ├── index.ts       # Chat helpers (getFlowPrompt, getScriptPrompt, …)
    ├── skills/        # One SKILL.md per CLI skill (cli render)
    ├── sdks/          # SDK reference extracted from the TypeScript and Python clients
    └── cli/           # CLI command reference extracted from cli/src/commands/
```

## How a topic is assembled

`TOPICS` in `generate.py` lists, for each topic, the parts both consumers concatenate in order:
a file under `base/` or `languages/`, or a token filled per consumer (`{lang}`, `{sdk}`,
`{wac_sdk}`, `{openflow_schema}`, `{cli_commands}`). The same entry produces the topic's chat
helper in `index.ts` and its CLI skill, so a part added there reaches both. A part tagged
`('chat', …)` or `('cli', …)` goes to that consumer only, and `cli_intro` heads the CLI skill with
a CLI-only workflow file (`flow-cli.md`, `script-cli.md`, `raw-app-cli.md`).

| Topic | Chat helper | CLI skill | Shared parts |
|---|---|---|---|
| script | `getScriptPrompt(lang)` | `write-script-<lang>` | `script-base.md`, the language file, its SDK |
| flow | `getFlowPrompt()` | `write-flow` | `flow-base.md`, the OpenFlow schema |
| raw app | `getRawAppPrompt(lang)` | `raw-app` | `raw-app.md` (chat adds the SDK) |
| resources | `getResourcePrompt()` | `resources` | `resources.md` |
| workflow-as-code | `getWorkflowAsCodePrompt(lang)` | `write-workflow-as-code` | `workflow-as-code.md`, the WAC SDK |
| pipeline | `getPipelinePrompt()` | `write-pipeline` | `pipeline-base.md` |
| triggers, schedules, preview, CLI commands | — | same names | CLI only |

The app chat embeds `RAW_APP_BASE` directly (`chat/app/core.ts`).

## Consumer fences

A sentence that only one consumer should see stays in the shared file, fenced:

```md
A tool name outside that character set fails every run of the flow.
<!-- chat-only -->
The flow write tools refuse such a name.
<!-- /chat-only -->
<!-- cli-only -->
`wmill lint <flow folder>` reports it before anything runs.
<!-- /cli-only -->
```

`render_for` (`utils.py`) renders every file once per consumer: `prompts.ts` gets the chat render,
the skills get the cli render, and the fence lines reach neither. Fences sit on their own lines and
cannot nest; an unbalanced or misspelled fence fails generation.

Use a fence for a sentence or a section. When most of a topic differs per consumer, a CLI-only
file used as `cli_intro` reads better than a file that is mostly fences.

## What goes where

- **Here:** anything true of Windmill regardless of who is asking — module shapes, data flow
  rules, what the editor accepts, how data tables, secrets or app access work.
- **In the chat's TypeScript prompt builders** (`frontend/src/lib/components/copilot/chat/*/core.ts`):
  only tool plumbing (which tool to call, its arguments) and values known at run time (the user's
  name, this app's data-table policy, session capabilities).
- **In `cli/src/guidance/core.ts`** (`AGENTS.wmill.md`): project-level CLI workflow — which skill
  to use, deploying, debugging jobs.

A file in `TOPICS` names no chat tool, not even inside a `chat-only` fence: it reaches every chat
mode (flow mode's system prompt, global mode's `get_instructions`), each with its own tool names,
and `global/sessionToolset.test.ts` fails a prompt that names a tool its session lacks. Describe the
action instead ("the flow write tools refuse such a name") and name the tool in TypeScript. The one
exception is `flow-chat-special-modules.md`, which only flow mode reads.

## Regenerating

After editing anything here, or when SDK methods, the OpenFlow schema or CLI commands change:

```bash
python system_prompts/generate.py
```

CI (`.github/workflows/check-system-prompts.yml`) runs `system_prompts/check-freshness.sh`, which
fails when the committed output is stale.

To also refresh the standalone skills in a Claude plugin checkout:

```bash
python system_prompts/generate.py --plugin-dir ~/windmill-claude-plugin
```

`--plugin-dir` accepts:
- the `windmill-claude-plugin` repo root
- a plugin root such as `plugins/windmill`
- a direct `skills/` directory

To regenerate the public docs repo (consumed by context7):

```bash
python system_prompts/generate.py --context7-dir ~/windmill-cli-docs
```

`--context7-dir` writes a fully-rendered snapshot (`AGENTS.md`,
`cli-commands.md`, `skills/<name>/SKILL.md`, `README.md`, `manifest.json`
with the Windmill `version`) with all template placeholders resolved —
suitable for ingestion by docs aggregators. In CI this runs from
`.github/workflows/publish-cli-docs.yml` on every release tag. The
generator refuses to wipe the target directory unless it's empty or has
a context7 marker (`context7.json`, `manifest.json`, or a
`windmill-cli-docs` git remote), so a typo can't delete unrelated files.
