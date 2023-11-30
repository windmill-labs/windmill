import * as abortSignal from "ext:deno_web/03_abort_signal.js";
import * as base64 from "ext:deno_web/05_base64.js";
import * as console from "ext:deno_console/01_console.js";
import DOMException from "ext:deno_web/01_dom_exception.js";
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
import * as urlPattern from "ext:deno_url/01_urlpattern.js";
import * as webidl from "ext:deno_webidl/00_webidl.js";
import * as response from "ext:deno_fetch/23_response.js";
import * as request from "ext:deno_fetch/23_request.js";
import "ext:deno_web/02_structured_clone.js";
import "ext:deno_web/04_global_interfaces.js";
import "ext:deno_web/13_message_port.js";
import "ext:deno_web/14_compression.js";
import "ext:deno_web/15_performance.js";

globalThis.atob = base64.atob;
globalThis.btoa = base64.btoa;
globalThis.fetch = fetch.fetch;
globalThis.Request = request.Request;
globalThis.Blob = file.Blob;
globalThis.URL = url.URL;
globalThis.FormData = formData.FormData;
globalThis.URLSearchParams = url.URLSearchParams;
globalThis.Headers = headers.Headers;
globalThis.FileReader = fileReader.FileReader;
globalThis.console = new console.Console((msg, level) =>
  globalThis.Deno.core.ops.op_log([msg])
);
// Object.assign(globalThis, {
//   console: nonEnumerable(
//     new console.Console((msg, level) => core.print(msg, level > 1))
//   ),

//   // timers
//   clearInterval: writable(timers.clearInterval),
//   clearTimeout: writable(timers.clearTimeout),
//   setInterval: writable(timers.setInterval),
//   setTimeout: writable(timers.setTimeout),

//   // fetch
//   Request: nonEnumerable(request.Request),
//   Response: nonEnumerable(response.Response),
//   Headers: nonEnumerable(headers.Headers),
//   fetch: writable(fetch.fetch),

//   // base64
//   atob: writable(base64.atob),
//   btoa: writable(base64.btoa),

//   // encoding
//   TextDecoder: nonEnumerable(encoding.TextDecoder),
//   TextEncoder: nonEnumerable(encoding.TextEncoder),
//   TextDecoderStream: nonEnumerable(encoding.TextDecoderStream),
//   TextEncoderStream: nonEnumerable(encoding.TextEncoderStream),

//   // url
//   URL: nonEnumerable(url.URL),
//   URLPattern: nonEnumerable(urlPattern.URLPattern),
//   URLSearchParams: nonEnumerable(url.URLSearchParams),

//   //   // crypto
//   //   CryptoKey: nonEnumerable(crypto.CryptoKey),
//   //   crypto: readOnly(crypto.crypto),
//   //   Crypto: nonEnumerable(crypto.Crypto),
//   //   SubtleCrypto: nonEnumerable(crypto.SubtleCrypto),

//   // streams
//   ByteLengthQueuingStrategy: nonEnumerable(streams.ByteLengthQueuingStrategy),
//   CountQueuingStrategy: nonEnumerable(streams.CountQueuingStrategy),
//   ReadableStream: nonEnumerable(streams.ReadableStream),
//   ReadableStreamDefaultReader: nonEnumerable(
//     streams.ReadableStreamDefaultReader
//   ),
//   ReadableByteStreamController: nonEnumerable(
//     streams.ReadableByteStreamController
//   ),
//   ReadableStreamBYOBReader: nonEnumerable(streams.ReadableStreamBYOBReader),
//   ReadableStreamBYOBRequest: nonEnumerable(streams.ReadableStreamBYOBRequest),
//   ReadableStreamDefaultController: nonEnumerable(
//     streams.ReadableStreamDefaultController
//   ),
//   TransformStream: nonEnumerable(streams.TransformStream),
//   TransformStreamDefaultController: nonEnumerable(
//     streams.TransformStreamDefaultController
//   ),
//   WritableStream: nonEnumerable(streams.WritableStream),
//   WritableStreamDefaultWriter: nonEnumerable(
//     streams.WritableStreamDefaultWriter
//   ),
//   WritableStreamDefaultController: nonEnumerable(
//     streams.WritableStreamDefaultController
//   ),

//   // event
//   CloseEvent: nonEnumerable(event.CloseEvent),
//   CustomEvent: nonEnumerable(event.CustomEvent),
//   ErrorEvent: nonEnumerable(event.ErrorEvent),
//   Event: nonEnumerable(event.Event),
//   EventTarget: nonEnumerable(event.EventTarget),
//   MessageEvent: nonEnumerable(event.MessageEvent),
//   PromiseRejectionEvent: nonEnumerable(event.PromiseRejectionEvent),
//   ProgressEvent: nonEnumerable(event.ProgressEvent),
//   reportError: writable(event.reportError),
//   DOMException: nonEnumerable(DOMException),

//   // file
//   Blob: nonEnumerable(file.Blob),
//   File: nonEnumerable(file.File),
//   FileReader: nonEnumerable(fileReader.FileReader),

//   // form data
//   FormData: nonEnumerable(formData.FormData),

//   // abort signal
//   AbortController: nonEnumerable(abortSignal.AbortController),
//   AbortSignal: nonEnumerable(abortSignal.AbortSignal),

//   //   // web sockets
//   //   WebSocket: nonEnumerable(webSocket.WebSocket),

//   //   // performance
//   //   Performance: nonEnumerable(performance.Performance),
//   //   PerformanceEntry: nonEnumerable(performance.PerformanceEntry),
//   //   PerformanceMark: nonEnumerable(performance.PerformanceMark),
//   //   PerformanceMeasure: nonEnumerable(performance.PerformanceMeasure),
//   //   performance: writable(performance.performance),

//   // messagePort
//   //   structuredClone: writable(messagePort.structuredClone),

//   // Branding as a WebIDL object
//   [webidl.brand]: nonEnumerable(webidl.brand),
// });

function nonEnumerable(value) {
  return {
    value,
    writable: true,
    enumerable: false,
    configurable: true,
  };
}

function writable(value) {
  return {
    value,
    writable: true,
    enumerable: true,
    configurable: true,
  };
}

function readOnly(value) {
  return {
    value,
    enumerable: true,
    writable: false,
    configurable: true,
  };
}

// function getterOnly(getter) {
//   return {
//     get: getter,
//     set() {},
//     enumerable: true,
//     configurable: true,
//   };
// }

// function formatException(error) {
//   if (ObjectPrototypeIsPrototypeOf(ErrorPrototype, error)) {
//     return null;
//   } else if (typeof error == "string") {
//     return `Uncaught ${console.inspectArgs([console.quoteString(error)], {
//       colors: false,
//     })}`;
//   } else {
//     return `Uncaught ${console.inspectArgs([error], { colors: false })}`;
//   }
// }
