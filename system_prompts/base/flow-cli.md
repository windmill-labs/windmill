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

After writing, act on the user's intent instead of just listing commands. Run `wmill flow preview` yourself when it fits (see "After writing — offer to run, don't wait passively" below). `wmill generate-metadata` regenerates local lock/hash files (not a deploy) but re-resolves deps — offer it and run on agreement, unless the project's `AGENTS.md` opts into running metadata automatically. Only *name* `wmill sync push` (the deploy) so the user can approve it. The options:

- `wmill flow preview <flow_path>` — **default when iterating on a local flow.** Runs the local `flow.yaml` against local inline scripts without deploying. Add `--remote` to use deployed workspace scripts for PathScript steps instead of local files. Add `--step <step_id>` to run only one module in isolation (see "Single-step vs whole-flow preview" below).
- `wmill flow run <path>` — runs the flow **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — regenerate stale local `.lock` files for the flow and its inline scripts and refresh their content hashes in `wmill-lock.yaml`. Writes local files only (not a deploy). Run it after editing inline scripts whose imports or arguments changed, so `wmill-lock.yaml` doesn't drift and add noise to git-sync/CI. By default it scans **scripts, flows, and apps** across the workspace but only regenerates stale ones; pass the flow's folder as an argument (or run from that subdirectory) to limit the scope to the flow you edited. Note a flow (or script) that imports a changed shared script is pulled in too — run `wmill generate-metadata --dry-run` to see exactly what is stale and why (`content changed` vs `depends on <path>`) before applying.
- Deploy local changes to the workspace — via `git push` or `wmill sync push` depending on how the repo is wired (see the **Deploying** section in `AGENTS.wmill.md`). Only suggest/run a deploy when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the flow", "try it", "test it", "does it work" while there are **local edits to a `flow.yaml`**, use `flow preview`. Do NOT push the flow to then `flow run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `flow run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local `flow.yaml` being edited (you're just invoking an existing flow).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### Single-step vs whole-flow preview

Use `flow preview <flow_path> --step <step_id>` when the user is iterating on one module and the flow's upstream steps aren't part of what they're trying to validate. It runs only that step's runnable (rawscript: the inline script; script: the PathScript, locally if available; flow: the subflow by path) and is much faster than running the whole flow when previous steps are slow or expensive. The step id is resolved by walking nested branchone/branchall/forloopflow/whileloopflow modules and includes the special `preprocessor` and `failure` modules.

Use `flow preview <flow_path>` (no `--step`) when steps depend on each other's outputs, when the user is validating the overall control flow, or when `--step` doesn't apply (branchone, branchall, forloopflow, whileloopflow, identity, and AI agent steps cannot themselves be tested in isolation — for branchone/branchall/forloopflow/whileloopflow, the *contained* steps can, by passing the inner step's id).

### After writing — offer to run, don't wait passively

This is about **programmatic execution** (`wmill flow preview -d '<args>'`), which actually runs the flow and has side effects. Visual preview (the `preview` skill) is offered separately — see "Visual preview" below.

If the user hasn't already told you to run/test the flow, offer it as a one-sentence next step (e.g. "Want me to run `wmill flow preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the flow in their original request, skip the offer and just execute `wmill flow preview <path> -d '<args>'` directly — pick plausible args from the flow's input schema.

`wmill flow preview` is safe to run yourself (it does not deploy). `wmill generate-metadata` does not deploy either (it only writes local lock/hash files) but re-resolves deps — offer it and run on agreement, unless the project's `AGENTS.md` opts into automatic metadata. After running it, check the regenerated `.lock` diff and tell the user which inline-script dependency versions changed, so they can catch an unwanted bump before deploying. Only `wmill sync push` deploys; run it only when the user explicitly asks.

### Visual preview

To open the flow visually in the dev page (graph + live reload), use the `preview` skill. Always **offer** it as a one-sentence next step (e.g. "Want me to open the visual preview?") rather than opening it automatically — opening the dev page has side effects (browser window, possibly a `launch.json` entry under MCP-preview branches) the user should consent to. If the user already asked to see/preview/visualize the flow in their original request, skip the offer and just invoke the skill.
