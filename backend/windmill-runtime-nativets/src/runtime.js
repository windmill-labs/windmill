import * as abortSignal from "ext:deno_web/03_abort_signal.js";
import * as domException from "ext:deno_web/01_dom_exception.js";
import * as base64 from "ext:deno_web/05_base64.js";
import * as console from "ext:deno_console/01_console.js";
import * as encoding from "ext:deno_web/08_text_encoding.js";
import * as event from "ext:deno_web/02_event.js";
import * as fetch from "ext:deno_fetch/26_fetch.js";
import * as file from "ext:deno_web/09_file.js";
import * as fileReader from "ext:deno_web/10_filereader.js";
import * as formData from "ext:deno_fetch/21_formdata.js";
import * as headers from "ext:deno_fetch/20_headers.js";
import * as streams from "ext:deno_web/06_streams.js";
import * as timers from "ext:deno_web/02_timers.js";
import * as url from "ext:deno_url/00_url.js";
import * as net from "ext:deno_net/01_net.js";
import * as tls from "ext:deno_net/02_tls.js";
import * as urlPattern from "ext:deno_url/01_urlpattern.js";
import * as webidl from "ext:deno_webidl/00_webidl.js";
import * as crypto from "ext:deno_crypto/00_crypto.js";
import * as response from "ext:deno_fetch/23_response.js";
import * as request from "ext:deno_fetch/23_request.js";
import "ext:deno_web/02_structured_clone.js";
import * as globalInterfaces from "ext:deno_web/04_global_interfaces.js";
// Namespace imports (not side-effect-only) so their constructors are reachable
// for the globalThis wiring below. The module bodies still execute on
// evaluation, so their side effects apply.
import * as messagePort from "ext:deno_web/13_message_port.js";
import * as compression from "ext:deno_web/14_compression.js";
import * as performance from "ext:deno_web/15_performance.js";
import "ext:deno_web/16_image_data.js";
import "ext:deno_fetch/27_eventsource.js";

globalThis.atob = base64.atob;
globalThis.btoa = base64.btoa;
globalThis.fetch = fetch.fetch;
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
globalThis.crypto = crypto.crypto;
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
globalThis.__wmInitPerIsolate = () => {
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
// We use dynamic import so deno_telemetry isn't loaded during snapshot creation.
// Config: [tracingEnabled, metricsEnabled, consoleConfig, deterministic]
// consoleConfig: 0=ignore, 1=capture, 2=replace
globalThis.__bootstrapOtel = () => {
  import("ext:deno_telemetry/telemetry.ts").then(({ bootstrap, enterSpan }) => {
    bootstrap([1, 0, 1, 0]);
    // Expose enterSpan for setting parent trace context
    globalThis.__enterSpan = enterSpan;
  });
};
