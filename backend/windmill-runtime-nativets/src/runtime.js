import { core, internals } from "ext:core/mod.js";

// deno_crypto brands CryptoKeys with this symbol and reads it from `internals`
// when it evaluates; in Deno a deno_node module sets it, which is not loaded here.
internals.kKeyObject ??= Symbol("kKeyObject");

// Every script loaded here runs at snapshot-build time and is baked into the
// snapshot. One that is first loaded at run time comes from the residual table
// build.rs emits instead.
const abortSignal = core.loadExtScript("ext:deno_web/03_abort_signal.js");
const domException = core.loadExtScript("ext:deno_web/01_dom_exception.js");
const base64 = core.loadExtScript("ext:deno_web/05_base64.js");
const console = core.loadExtScript("ext:deno_web/01_console.js");
const encoding = core.loadExtScript("ext:deno_web/08_text_encoding.js");
const event = core.loadExtScript("ext:deno_web/02_event.js");
const fetch = core.loadExtScript("ext:deno_fetch/26_fetch.js");
const file = core.loadExtScript("ext:deno_web/09_file.js");
const fileReader = core.loadExtScript("ext:deno_web/10_filereader.js");
const formData = core.loadExtScript("ext:deno_fetch/21_formdata.js");
const headers = core.loadExtScript("ext:deno_fetch/20_headers.js");
const streams = core.loadExtScript("ext:deno_web/06_streams.js");
const timers = core.loadExtScript("ext:deno_web/02_timers.js");
const url = core.loadExtScript("ext:deno_web/00_url.js");
const urlPattern = core.loadExtScript("ext:deno_web/01_urlpattern.js");
const webidl = core.loadExtScript("ext:deno_webidl/00_webidl.js");
const crypto = core.loadExtScript("ext:deno_crypto/00_crypto.js");
const response = core.loadExtScript("ext:deno_fetch/23_response.js");
const request = core.loadExtScript("ext:deno_fetch/23_request.js");
const globalInterfaces = core.loadExtScript(
  "ext:deno_web/04_global_interfaces.js",
);
const messagePort = core.loadExtScript("ext:deno_web/13_message_port.js");
const compression = core.loadExtScript("ext:deno_web/14_compression.js");
const performance = core.loadExtScript("ext:deno_web/15_performance.js");

// deno_fetch applies no deadline, so a peer that accepts a request and then
// never answers leaves `await fetch(...)` pending until the job timeout, which
// self-hosted defaults to 7 days.
const DENO_FETCH = fetch.fetch;

// deno_web's timers reject any `this` other than undefined/globalThis, so
// `timers.setTimeout(...)` passes the module namespace and throws "Illegal
// invocation".
const setTimeoutUnbound = timers.setTimeout;
const clearTimeoutUnbound = timers.clearTimeout;
// Captured before user code shares the isolate and could redefine them, the
// way deno's own modules reach intrinsics through primordials.
const PromiseReject = Promise.reject.bind(Promise);
const promiseThen = Function.prototype.call.bind(Promise.prototype.then);
const abortSignalAny = abortSignal.AbortSignal.any.bind(abortSignal.AbortSignal);
const abortControllerAbort = Function.prototype.call.bind(
  abortSignal.AbortController.prototype.abort,
);
const ReflectApply = Reflect.apply;

// Forwards the original argument count, like every caller below relies on. The
// extra promise hop under `no_network` costs the same-tick settling of an
// aborted fetch, which only matters where a request could have been made.
function ORIGINAL_FETCH() {
  const p = ReflectApply(DENO_FETCH, undefined, arguments);
  return noNetwork ? promiseThen(p, undefined, rewordNoNetworkDenial) : p;
}

// Installed per isolate by __wmInitPerIsolate; 0 disables. Only a backstop
// for the impossible case of fetch running before that init.
let fetchResponseTimeoutMs = 900_000;

function fetchResponseTimeoutError(requestUrl, timeoutMs) {
  let target;
  try {
    const parsed = new url.URL(requestUrl);
    // Query and fragment routinely carry tokens, and this reaches a job log.
    target = parsed.origin + parsed.pathname;
  } catch {
    target = "the request target";
  }
  return new domException.DOMException(
    `fetch to ${target} timed out: no response headers arrived within ` +
      `${Math.round(timeoutMs / 1000)}s (this covers connect, request upload ` +
      `and the wait for the server to start replying; once a response begins ` +
      `it is never interrupted). Change it per script with ` +
      `"//fetch_response_timeout <seconds>" (0 disables), or instance-wide ` +
      `with WINDMILL_FETCH_RESPONSE_TIMEOUT_SECS.`,
    "TimeoutError",
  );
}

// Set per isolate for a `//no_network` script. The denial itself is the
// isolate's permissions (Rust); this only replaces deno's "run again with the
// --allow-net flag" hint, which names a flag a script author cannot pass.
let noNetwork = false;

function rewordNoNetworkDenial(e) {
  if (e?.name === "NotCapable" && typeof e.message === "string") {
    e.message = e.message.replace(
      ", run again with the --allow-net flag",
      ": the script is annotated //no_network",
    );
  }
  throw e;
}

globalThis.atob = base64.atob;
globalThis.btoa = base64.btoa;
// Not `async`, for the same reason deno_fetch's own outer fetch isn't: WPT
// pins that an aborted fetch settles in the same tick, which adopting its
// promise through another one would break. Construction still has to reject
// rather than throw, so it is caught and handed back as a rejection.
globalThis.fetch = function fetch(input, init = undefined) {
  const timeoutMs = fetchResponseTimeoutMs;
  // Forwarded with the original argument count, so deno still sees an empty
  // call as empty and raises its own "1 argument required". The default on
  // `init` is what keeps `fetch.length` at 1, as the standard has it.
  if (!(timeoutMs > 0) || arguments.length < 1) {
    return ReflectApply(ORIGINAL_FETCH, undefined, arguments);
  }

  let req;
  let controller;
  let signal;
  try {
    // RequestInit is a WebIDL dictionary: copying it drops inherited and
    // non-enumerable members, and inheriting from it runs accessors against the
    // wrong receiver. Hand it to the same Request constructor fetch() would,
    // and carry our own signal in an init of our own.
    req = new request.Request(input, init);

    // `req.signal` is deno's own resolution of init.signal over an input
    // Request's signal, so combining with it preserves the caller's abort and
    // reason while ours only adds a ceiling.
    controller = new abortSignal.AbortController();
    signal = abortSignalAny([req.signal, controller.signal]);
  } catch (e) {
    return PromiseReject(e);
  }

  // Already aborted: hand back deno's own settled rejection untouched, and arm
  // nothing -- there is no response to wait for.
  if (signal.aborted) {
    return ORIGINAL_FETCH(req, { signal });
  }

  let timer = setTimeoutUnbound(() => {
    timer = undefined;
    abortControllerAbort(controller, fetchResponseTimeoutError(req.url, timeoutMs));
  }, timeoutMs);
  // Disarmed on headers, never on body completion: a response that has begun
  // arriving must be free to stream for as long as it needs.
  const disarm = () => {
    if (timer !== undefined) {
      clearTimeoutUnbound(timer);
      timer = undefined;
    }
  };

  return promiseThen(
    ORIGINAL_FETCH(req, { signal }),
    (res) => {
      disarm();
      return res;
    },
    (e) => {
      disarm();
      throw e;
    },
  );
};
globalThis.Request = request.Request;
globalThis.Response = response.Response;
globalThis.Blob = file.Blob;
globalThis.URL = url.URL;
globalThis.FormData = formData.FormData;
globalThis.URLSearchParams = url.URLSearchParams;
globalThis.Headers = headers.Headers;
globalThis.FileReader = fileReader.FileReader;
globalThis.console = new console.Console((msg, level) =>
  globalThis.Deno.core.ops.op_log(msg)
);
globalThis.AbortController = abortSignal.AbortController;
globalThis.AbortSignal = abortSignal.AbortSignal;
// A getter, not a value: `crypto.crypto` mints a cppgc object, and the cppgc
// heap is not attached while this module runs at snapshot-build time.
Object.defineProperty(globalThis, "crypto", {
  configurable: true,
  enumerable: true,
  get: () => crypto.crypto,
  // Assignment has to keep working, as it does on a plain property.
  set(value) {
    Object.defineProperty(globalThis, "crypto", {
      configurable: true,
      enumerable: true,
      writable: true,
      value,
    });
  },
});
globalThis.Crypto = crypto.Crypto;
globalThis.CryptoKey = crypto.CryptoKey;
globalThis.SubtleCrypto = crypto.SubtleCrypto;

Object.assign(globalThis, {
  clearInterval: timers.clearInterval,
  clearTimeout: timers.clearTimeout,
  setInterval: timers.setInterval,
  setTimeout: timers.setTimeout,
});

// Standard web-platform globals from the deno_web / deno_url extensions,
// exposed to match the bun runner's global surface. Every name below is present
// in bun; names bun lacks (EventSource, ImageData) are deliberately excluded.
Object.assign(globalThis, {
  // DOMException. Beyond bun parity, deno_web modules reference it as a global:
  // AbortController.abort() with no reason constructs `new DOMException(...)`, so
  // without this the already-wired AbortController/AbortSignal throw on abort.
  DOMException: domException.DOMException,
  // Text encoding + encoding streams.
  TextEncoder: encoding.TextEncoder,
  TextDecoder: encoding.TextDecoder,
  TextEncoderStream: encoding.TextEncoderStream,
  TextDecoderStream: encoding.TextDecoderStream,
  // File (Blob is already wired above).
  File: file.File,
  // Events (AbortSignal, already wired, extends EventTarget). MessageEvent is
  // the companion to MessagePort/MessageChannel below. Only the event types
  // bun exposes are wired (ProgressEvent / PromiseRejectionEvent are not).
  // reportError works because __wmInitPerIsolate makes globalThis an EventTarget.
  Event: event.Event,
  EventTarget: event.EventTarget,
  CustomEvent: event.CustomEvent,
  MessageEvent: event.MessageEvent,
  CloseEvent: event.CloseEvent,
  ErrorEvent: event.ErrorEvent,
  reportError: event.reportError,
  // Streams + queuing strategies + the reader/controller constructors bun also
  // exposes as globals (used for `x instanceof ReadableStreamDefaultReader` etc.;
  // the controllers throw on direct construction, matching the spec).
  ReadableStream: streams.ReadableStream,
  ReadableStreamDefaultReader: streams.ReadableStreamDefaultReader,
  ReadableStreamBYOBReader: streams.ReadableStreamBYOBReader,
  ReadableStreamDefaultController: streams.ReadableStreamDefaultController,
  ReadableByteStreamController: streams.ReadableByteStreamController,
  ReadableStreamBYOBRequest: streams.ReadableStreamBYOBRequest,
  WritableStream: streams.WritableStream,
  WritableStreamDefaultWriter: streams.WritableStreamDefaultWriter,
  WritableStreamDefaultController: streams.WritableStreamDefaultController,
  TransformStream: streams.TransformStream,
  TransformStreamDefaultController: streams.TransformStreamDefaultController,
  ByteLengthQueuingStrategy: streams.ByteLengthQueuingStrategy,
  CountQueuingStrategy: streams.CountQueuingStrategy,
  // URL pattern matching.
  URLPattern: urlPattern.URLPattern,
  // Compression streams.
  CompressionStream: compression.CompressionStream,
  DecompressionStream: compression.DecompressionStream,
  // Message channel / port.
  MessageChannel: messagePort.MessageChannel,
  MessagePort: messagePort.MessagePort,
  // High-resolution timing: the `performance` singleton and its constructor
  // globals (bun exposes all of these; PerformanceObserver is not in deno_web).
  performance: performance.performance,
  Performance: performance.Performance,
  PerformanceEntry: performance.PerformanceEntry,
  PerformanceMark: performance.PerformanceMark,
  PerformanceMeasure: performance.PerformanceMeasure,
  // Spec structuredClone (validates args + honors the options bag), from the
  // message-port module rather than the single-arg internal helper in
  // 02_structured_clone.js.
  structuredClone: messagePort.structuredClone,
});

// Per-isolate init, invoked from Rust after the snapshot is restored (this
// module body runs at snapshot-build time, not per isolate).
globalThis.__wmInitPerIsolate = (config) => {
  if (config != null && typeof config.fetchResponseTimeoutMs === "number") {
    fetchResponseTimeoutMs = config.fetchResponseTimeoutMs;
  }
  noNetwork = config?.noNetwork === true;

  // setTimeOrigin() seeds performance.timeOrigin from the isolate's wall clock;
  // without it timeOrigin is undefined and `timeOrigin + performance.now()` is NaN.
  performance.setTimeOrigin();

  // Make globalThis a functional EventTarget, as Deno's bootstrap does. deno_web
  // routes uncaught EventTarget-listener errors and reportError through
  // reportException, which dispatches an error event on the saved global
  // reference; reportError also requires its receiver to equal that reference.
  // Both need the reference to be globalThis and globalThis to be an EventTarget,
  // otherwise dispatch throws a masking error and globalThis.reportError() throws
  // "Illegal invocation". Set up per isolate so the reference is the live global.
  // The prototype + brand are what webidl.assertBranded checks in the methods.
  Object.setPrototypeOf(
    globalThis,
    globalInterfaces.DedicatedWorkerGlobalScope.prototype,
  );
  event.setEventTargetData(globalThis);
  globalThis[webidl.brand] = webidl.brand;
  event.saveGlobalThisReference(globalThis);
};

// Expose bootstrapOtel globally so it can be called from Rust after runtime creation.
// Loaded on first call so deno_telemetry isn't evaluated during snapshot creation.
// Config: [tracingEnabled, metricsEnabled, consoleConfig, deterministic]
// consoleConfig: 0=ignore, 1=capture, 2=replace
globalThis.__bootstrapOtel = () => {
  const { bootstrap, enterSpan } = core.loadExtScript(
    "ext:deno_telemetry/telemetry.ts",
  );
  bootstrap([1, 0, 1, 0]);
  // Expose enterSpan for setting parent trace context
  globalThis.__enterSpan = enterSpan;
};
