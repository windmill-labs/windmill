# Windmill Flow CLI Guide

## Creating a Flow

**You — the AI agent — scaffold the flow yourself by running `wmill flow new <path>` with the right flags. Do NOT hand-create the folder + `flow.yaml`, and do NOT tell the user to "run `wmill flow new` and follow the prompts".**

`wmill flow new` creates the folder with the correct suffix (`__flow` or `.flow` depending on the workspace's `nonDottedPaths` setting), writes a minimal `flow.yaml` shell, and prints Claude-specific next-step hints. Scaffolding by hand skips all of that and often picks the wrong suffix.

### Step 1 — Gather path + summary by asking the user

You need two things:

1. **path** — the windmill path, e.g. `f/folder/my_flow` or `u/username/my_flow`.
2. **summary** — a short description of the flow.

If the user's request didn't supply both, ask for both in a single round-trip. Use whichever interactive question facility your runtime provides — a structured multi-choice tool if available, otherwise plain chat — and provide one or two example values for each (with an "Other" / free-form fallback). Do not guess paths or summaries.

### Step 2 — Run the command yourself

```bash
wmill flow new f/folder/my_flow --summary "Short description"
```

Add `--description "..."` when the user provided a longer explanation worth preserving separately from the summary.

### Step 3 — Fill in `flow.yaml`

Open the generated `flow.yaml` (under the folder the command just created) and replace the empty `value.modules` + `schema` with the real flow definition.

For rawscript modules, use `!inline path/to/script.ts` for the content key. Inline script files should NOT include `.inline_script.` in their names (e.g. use `a.ts`, not `a.inline_script.ts`).

Once the flow has real content, **offer** to open the visual preview as a one-sentence next step (e.g. "Want me to open the visual preview?"). Don't auto-open — opening the dev page has side effects (browser window, possibly a `launch.json` entry) and the user should consent.

### Anti-patterns to avoid

- ❌ Hand-creating the `__flow` folder + `flow.yaml` instead of running `wmill flow new`. You'll miss the suffix-setting resolution, the default shape, and the Claude hints.
- ❌ Telling the user to "run `wmill flow new <path>`" — you can and should run it yourself.
- ❌ Inventing a path/summary instead of asking the user.

## CLI Commands — running, previewing, deploying

After writing, tell the user which command fits what they want to do:

- `wmill flow preview <flow_path>` — **default when iterating on a local flow.** Runs the local `flow.yaml` against local inline scripts without deploying. Add `--remote` to use deployed workspace scripts for PathScript steps instead of local files.
- `wmill flow run <path>` — runs the flow **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — regenerate stale `.lock` and `.script.yaml` files. By default it scans **scripts, flows, and apps** across the workspace; pass `--skip-flows --skip-apps` (or run from a subdirectory) to limit the scope when you only care about the flow you edited.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the flow", "try it", "test it", "does it work" while there are **local edits to a `flow.yaml`**, use `flow preview`. Do NOT push the flow to then `flow run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `flow run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local `flow.yaml` being edited (you're just invoking an existing flow).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### After writing — offer to run, don't wait passively

This is about **programmatic execution** (`wmill flow preview -d '<args>'`), which actually runs the flow and has side effects. Visual preview (the `preview` skill) is offered separately — see "Visual preview" below.

If the user hasn't already told you to run/test the flow, offer it as a one-sentence next step (e.g. "Want me to run `wmill flow preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the flow in their original request, skip the offer and just execute `wmill flow preview <path> -d '<args>'` directly — pick plausible args from the flow's input schema.

`wmill flow preview` is safe to run yourself (it does not deploy). `wmill sync push` and `wmill generate-metadata` modify workspace state or local files — only run these when the user explicitly asks; otherwise tell them which to run.

### Visual preview

To open the flow visually in the dev page (graph + live reload), use the `preview` skill. Always **offer** it as a one-sentence next step (e.g. "Want me to open the visual preview?") rather than opening it automatically — opening the dev page has side effects (browser window, possibly a `launch.json` entry under MCP-preview branches) the user should consent to. If the user already asked to see/preview/visualize the flow in their original request, skip the offer and just invoke the skill.
