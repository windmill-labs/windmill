# What an item reads, and who reads it

`POST /w/{workspace}/access/reads` (`backend/windmill-api-groups/src/access_reads.rs`) returns
what a script, flow or app reads when it runs, with each read credited to the identity the
runtime performs it as. Two features take a slice of that answer
(`frontend/src/lib/components/access/detectNeeds.ts`):

- The access view's "What it reads" (`neededByGrantee`) lists what a member needs to run the
  item: what whoever opens it reads, the item's own reads when it runs as its caller or as that
  member, and the reads of a nested runnable that runs as that member.
- The run-as settings (`neededByRunAs`) list what an account needs when the item runs as it: the
  item's own reads and the reads of a nested runnable that runs as that account.

## Who performs each read

The scan mirrors the runtime. A change to who a step, a runnable or a task runs as must change
`tag` in `access_reads.rs` too.

| Code that reads | Reads as |
|---|---|
| A script or flow with no `on_behalf_of` | whoever starts it |
| A script or flow with `on_behalf_of` | that account |
| A flow's inline step, and the JavaScript of its input transforms (`variable('…')`) | the flow's runner |
| A static `$res:`/`$var:` argument | the job that receives it: the flow's runner for an inline step, the step's own runner for a path step |
| A flow step calling a script or flow by path | the target's `on_behalf_of` if set, else the flow's runner; the step needs no read on its target |
| A workflow-as-code `taskScript`/`taskFlow`, an AI agent's tool | like a flow step |
| An AI agent's MCP server and its linked `agent` resource | the agent step's job |
| An SDK `runScriptByPath`/`runFlow` call | the calling job, which needs read on the target; the target then runs as its own `on_behalf_of` if set |
| An app runnable in `publisher`, `guest` or `anonymous` mode, inline or by path | the policy's `on_behalf_of`; a called script's own `on_behalf_of` is dropped |
| An app runnable in `viewer` mode | whoever opens the app |
| A full-code app's frontend files (`value.files`) | whoever opens the app, in every mode |
| A `$res:`/`$var:` link inside a resource's value | whoever reads the resource |

An app in `publisher`, `guest` or `anonymous` mode whose policy has no `on_behalf_of` fails every
run until it is redeployed. The response's `runs_as` then has no principal, and `errors` says so.

## How it reads

- Every item is fetched under the caller's RLS and token scopes, in `user_db` transactions, so
  the answer never shows more than the caller can open. The handler takes no `DB`, which keeps a
  plain-pool read from being added by accident. An item that is missing and one the caller cannot
  read give the same error line.
- A script called by path is read at the version a run starts: its newest version with a lock,
  else its newest. A step pinned to a hash reads that hash.
- Code is parsed by the parsers the deploy-time asset inference uses
  (`windmill-api-scripts/src/body_parse.rs`), which return the item references beside the assets.
  A language with no parser, and code that fails to parse, is scanned lexically for literal SDK
  calls.
- A resource asset's access type describes the data behind the resource, and reaching that data
  takes read on the resource either way. A write comes only from `setResource`/`setVariable`.
- Assets that are not workspace items, such as data tables and object storage files, are listed
  under `others`: item grants do not cover them.
- Not found: a path built while the item runs, passed through a helper, or read in a module the
  item imports.
- Limits: 4 levels of steps, SDK runs and agents (resource links add none), 500 items, and 1 MiB
  of code parsed per file; past that, code is scanned lexically.
