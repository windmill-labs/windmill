# Solving a task with code

When the user asks for something done against the systems their workspace connects to (look
something up in an API, move data, check a status), answer it with a Windmill script run as a
preview: it executes on the workspace's workers, with its resources. A script that worked can be
saved and run as is the next time.

1. **Look for a saved script first**, by the user's words and by the system the task touches. If
   one fits, run it rather than writing new code; if one nearly fits, offer to change it rather
   than writing a near-duplicate.
2. **Otherwise write one script and run it as a preview.** Do the fetching, filtering and
   combining in the code, and return only what answers the task, not raw API payloads. Take
   credentials as a resource-typed `main` parameter, passed as `$res:<path>` when running, never
   as a token in the code. Give `main` typed parameters for whatever would change between runs,
   so the script is reusable.
3. **Offer to save it** when the task is likely to come back, without saving it unasked. Saving
   makes a draft, deployed only when the user asks; from then on it is an ordinary script.

<!-- chat-only -->
For a third-party API, look up its hub integration for its resource type and how to call it.
<!-- /chat-only -->

<!-- cli-only -->
Load the `write-script-<language>` skill before writing the script; the resource's fields are in
`rt.d.ts` (`wmill resource-type generate-namespace`). A preview refuses a file that
is not at a workspace path inside the project: write it at `u/<username>/<name>.ts` (`wmill
workspace whoami` gives the username) without asking, never under `f/`, which a push deploys,
even next to the resource it uses; run it with `wmill script preview
<file> -d '<args>'`. Search with `wmill script list` and `wmill flow list`; run a deployed one
with `wmill script run <path>`. Never push the file on your own. Delete it if the user does not
keep it; if they do, move it where the project syncs (the `includes` of `wmill.yaml`, `f/**` by
default) and push only on their go.
<!-- /cli-only -->
