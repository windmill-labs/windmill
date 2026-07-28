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
  const thrown = e as any;
  // The three named fields are read off whatever was thrown, exactly as the
  // bun/deno wrappers read them for a failed child job — `e.name`, `e.message`,
  // `e.stack`, whether or not it is an `Error`. Anything else here lets a
  // handler tell a task from a step in the fields it is told to branch on: a
  // thrown `{ name, message, code }` keeps all three as a task, and a
  // `class MyError extends Error {}` that never assigns `this.name` reports
  // `Error` as a task, so both must here too. What is absent stays absent and
  // the backend fills it, which is what a task with no usable fields gets.
  const name = typeof thrown?.name === "string" ? thrown.name : undefined;
  const message = typeof thrown?.message === "string" ? thrown.message : undefined;
  const stack = typeof thrown?.stack === "string" ? thrown.stack : undefined;
  const error: Record<string, any> = {};
  if (name) error.name = name;
  if (message !== undefined) error.message = message;
  if (stack) error.stack = stack;
  // Custom properties go under `extra`, the same key and the same skip-list the
  // bun/deno executors use for a failed child job, so an error carrying e.g. a
  // `code` keeps it whether it failed as a task or as a step.
  //
  // Unlike the executors, which serialize once as the job dies, this record is
  // stringified while the workflow is still running — once for the checkpoint
  // POST and again for the wrapper's output. A property that cannot survive
  // that would take the whole workflow down instead of reaching the `catch` the
  // user wrote, so each one has to prove it round-trips before it is kept.
  // `AxiosError.request` is the everyday example: it is circular via
  // `socket._httpMessage`. Reading the property can throw too, since
  // `getOwnPropertyNames` returns accessors and this invokes them.
  // Objects only: on a primitive `Object.getOwnPropertyNames` returns its
  // character indices, and `extra: {0: "b", 1: "o", …}` is noise the checkpoint
  // would then have to carry.
  if (thrown !== null && typeof thrown === "object") {
    const extra: Record<string, any> = {};
    for (const k of Object.getOwnPropertyNames(thrown)) {
      if (SERIALIZED_ERROR_FIELDS.includes(k)) continue;
      try {
        const v = thrown[k];
        JSON.stringify(v);
        extra[k] = v;
      } catch {
        // unreadable or unserializable: the failure itself still gets reported
      }
    }
    if (Object.keys(extra).length > 0) error.extra = extra;
  }
  return { __wmill_error: true, message: message ?? String(e), step_key: key, result: { error } };
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
