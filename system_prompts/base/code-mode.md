# Solving a task with code

When the user asks for something done against the systems their workspace connects to (look
something up in an API, move data, check a status, post a message), the work is a Windmill script
run as a preview: it executes in the workspace, on its workers, with the workspace's resources.
A script that answered the task once can be saved and run as is the next time, by the user, by
you, by a schedule or by a flow.

## 1. Look for a saved script first

Search the workspace's scripts and flows for one that already does the task, by the words the
user used and by the system it touches. If one fits, read its inputs and run the deployed one
rather than writing new code. If one nearly fits, say so and offer to change it rather than
writing a near-duplicate next to it.

## 2. Otherwise write the code and run it as a preview

- One script, not a chain of small calls: fetch, filter and combine in the code, so that what
  comes back is the answer.
- Credentials come from the workspace's resources. Declare them as typed `main` parameters (a
  resource-typed argument) and pass `$res:<path>` when running; never paste a token into the code
  or the arguments. Look up which resources of the type exist; if none does, say what is missing.
- For a third-party API, look up its hub integration first for how to call it.
- Return only what the user needs: the matching rows, the counts, the ids, not the raw API
  payload. A large raw result wastes the context it lands in.
- Give `main` typed parameters for everything that would change from one run to the next (a
  repository, a date range, a channel) rather than hard-coding them, so the script is reusable
  once saved.
- If it fails, fix the code and run it again; read the logs before guessing.

<!-- cli-only -->
`wmill script preview` runs a file at a workspace path, so write the script in your own scope,
`u/<username>/<name>.ts` (`wmill workspace whoami` gives the username), without asking where:
that scope is the default for this kind of work, and the file is then already kept where a later
push deploys it. Ask only if the user wants it in a shared folder. Run it with
`wmill script preview u/<username>/<name>.ts -d '<args>'`. Search with `wmill script list` and
`wmill flow list`; run a deployed one with `wmill script run <path>`.
<!-- /cli-only -->

## 3. Offer to save what worked

When the preview answered the task and the task is likely to come back, offer to save it as a
script: a path in the user's own scope for themselves, or in a folder the team shares, a summary
saying what it does, and the typed inputs it already has. Saving makes a draft; it reaches the
workspace once deployed, which happens only when the user asks. Once deployed, it is an ordinary
script: the next time the same task comes up, run it directly.
