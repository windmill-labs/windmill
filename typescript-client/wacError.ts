// Workflow-as-Code failure record, client side.
//
// Deliberately dependency-free so the tests can import it directly: the rest of
// client.ts pulls in the generated API modules, which is why the workflow test
// suite re-implements WorkflowCtx inline. What a caught failure looks like, and
// what counts as a failure rather than the SDK's own control flow, is decided
// here, against the shipped code rather than a copy of it.

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

/** Read a property off a value that may fight back: a proxy `get` trap or a
 *  throwing accessor turns an ordinary field read into an exception. Callers
 *  are on the failure-reporting path, where nothing has been checkpointed yet. */
function safeRead(o: any, k: string): unknown {
  try {
    return o?.[k];
  } catch {
    return undefined;
  }
}

/** @internal
 *  Whether a caught value is the SDK's own suspend signal, which must be
 *  rethrown rather than reported as a step failure.
 *
 *  Both halves of the check can throw on a hostile value: `instanceof` consults
 *  a proxy's `getPrototypeOf` trap, and reading `.name` its `get` trap. This
 *  runs before anything has been checkpointed, so neither may escape — a value
 *  that fights back is simply not a suspend. */
export function isSuspendSignal(e: unknown, suspendCtor: Function): boolean {
  try {
    if (e instanceof (suspendCtor as any)) return true;
  } catch {
    // a proxy refusing to be inspected is not the SDK's own signal
  }
  return safeRead(e, "name") === "StepSuspend";
}

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
  //
  // `throw "boom"` therefore reports the backend's fallback message rather than
  // "boom", because a task throwing a string is equally lossy — its `e.message`
  // is undefined too. Recovering the text here, from the marker's top-level
  // `message`, would split `message` between a task and a step, which is the
  // thing this record exists to prevent. The executors are where a primitive
  // throw could keep its text for both.
  const rawName = safeRead(thrown, "name");
  const rawMessage = safeRead(thrown, "message");
  const rawStack = safeRead(thrown, "stack");
  const name = typeof rawName === "string" ? rawName : undefined;
  const message = typeof rawMessage === "string" ? rawMessage : undefined;
  const stack = typeof rawStack === "string" ? rawStack : undefined;
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
    // `ownKeys` is a proxy trap too, so even listing the properties can throw.
    let keys: string[] = [];
    try {
      keys = Object.getOwnPropertyNames(thrown);
    } catch {
      keys = [];
    }
    for (const k of keys) {
      if (SERIALIZED_ERROR_FIELDS.includes(k)) continue;
      try {
        // The snapshot, not the original: probing the original only proves it
        // serialized *once*. A `toJSON` that succeeds and then throws — or one
        // whose output depends on state that moves — would pass here and break
        // the checkpoint encoding afterwards, when the failure has nowhere left
        // to go. Keeping what the probe produced makes the check binding.
        // `undefined` means the key would be omitted anyway (a function, a
        // symbol, an explicit undefined), and `JSON.parse` rejects it for us.
        extra[k] = JSON.parse(JSON.stringify(thrown[k]));
      } catch {
        // unreadable or unserializable: the failure itself still gets reported
      }
    }
    if (Object.keys(extra).length > 0) error.extra = extra;
  }
  // `String(e)` throws in turn on a value with no `toString` to reach —
  // `Object.create(null)`, a proxy that rejects the coercion — and this runs
  // inside the catch that is reporting the user's failure, so an escape here
  // replaces their error with an unrelated one and skips the checkpoint.
  let fallback: string;
  try {
    fallback = String(e);
  } catch {
    fallback = `unrepresentable ${typeof e} thrown`;
  }
  return {
    __wmill_error: true,
    message: message ?? fallback,
    step_key: key,
    result: { error },
  };
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
