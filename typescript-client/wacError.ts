// Workflow-as-Code failure record, client side.
//
// Deliberately dependency-free so the tests can import it directly: the rest of
// client.ts pulls in the generated API modules, which is why the workflow test
// suite re-implements WorkflowCtx inline. The two functions below are the ones
// that decide what a caught failure looks like, so they are the ones that must
// be tested against the shipped code rather than against a copy of it.

/** Error properties the executors already report as named fields, so they must
 *  not be repeated inside `extra`. Kept identical to the skip-list in the
 *  generated bun/deno job wrappers. */
const SERIALIZED_ERROR_FIELDS = [
  "line",
  "name",
  "stack",
  "column",
  "message",
  "sourceURL",
  "originalLine",
  "originalColumn",
];

/** @internal
 *  Serialize a failed `step()` body into the `__wmill_error` marker that task
 *  failures also use, so it can be stored in `completed_steps`.
 *
 *  The record's final shape is decided by the backend (`wac_failure_record`),
 *  which normalizes task failures through the same function; what is built here
 *  is the raw material plus the envelope the backend recognizes. */
export function stepErrorMarker(key: string, e: unknown): Record<string, any> {
  const message = e instanceof Error ? e.message : String(e);
  // `e.name`, matching what the bun/deno executors record for a failed child
  // job. Reaching for the constructor name instead would report `MyError` for a
  // `class MyError extends Error {}` that never assigns `this.name` where the
  // same failure in a task() reports `Error`, in the one field a handler is
  // told it can branch on.
  const name = e instanceof Error ? e.name || e.constructor?.name : typeof e;
  const error: Record<string, any> = { name, message };
  const stack = e instanceof Error ? e.stack : undefined;
  if (stack) error.stack = stack;
  // Custom properties go under `extra`, the same key and the same skip-list the
  // bun/deno executors use for a failed child job, so an error carrying e.g. a
  // `code` keeps it whether it failed as a task or as a step.
  if (e instanceof Error) {
    const extra: Record<string, any> = {};
    for (const k of Object.getOwnPropertyNames(e)) {
      if (SERIALIZED_ERROR_FIELDS.includes(k)) continue;
      extra[k] = (e as any)[k];
    }
    if (Object.keys(extra).length > 0) error.extra = extra;
  }
  return { __wmill_error: true, message, step_key: key, result: { error } };
}

/** @internal
 *  Rebuild the error a failed task or step throws. The run that produced the
 *  failure and every later replay go through here: a catch is control flow,
 *  `workflow()` re-runs its body from the top every round, so a handler that
 *  branches on the failure it caught must be handed the same thing in every
 *  round or it dispatches different tasks on the way back. */
export function taskErrorFromMarker(marker: any, fallbackMessage: string): Error {
  const err = new Error(marker?.message || fallbackMessage);
  // Matches the python client, which raises TaskError here. Keeps a failed
  // job's serialized error identical across the two languages.
  err.name = "TaskError";
  (err as any).result = marker?.result;
  (err as any).step_key = marker?.step_key;
  (err as any).child_job_id = marker?.child_job_id;
  return err;
}
