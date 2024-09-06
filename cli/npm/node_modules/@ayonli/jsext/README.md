# JsExt

A JavaScript extension package for building strong and modern applications.

This package is built on top of modern web standards and provides unified
high-level APIs that can work across different runtime environments, whether
it's Node.js, Deno, Bun, Cloudflare Workers, browsers, Windows, macOS or Linux.

## Outstanding Features

- [x] Various useful functions for built-in data types that are not built-in.
- [x] Various utility functions to extend the ability of flow control.
- [x] Multi-threaded JavaScript with parallel threads.
- [x] File system APIs for both server and browser environments.
- [x] Open dialogs in both CLI and web applications.
- [x] Serve HTTP requests, SSE and WebSockets for all server runtimes.
- [x] Manipulate file system paths and URLs in the same fashion.
- [x] Process byte arrays and readable streams effortlessly.
- [x] Create, extract and preview archives in all runtimes.
- [x] And many more...

## Import

The recommended way is to only import the ones that are needed:

```js
// Universal
import _try from "@ayonli/jsext/try";
import func from "@ayonli/jsext/func";
// ...

// Deno (URL)
import _try from "https://lib.deno.dev/x/ayonli_jsext@latest/try.ts";
import func from "https://lib.deno.dev/x/ayonli_jsext@latest/func.ts";
// ...

// Browsers (URL)
import _try from "https://ayonli.github.io/jsext/esm/try.js";
import func from "https://ayonli.github.io/jsext/esm/func.js";
// ...
```

There is also a bundled version that can be loaded via a `<script>` tag in the
browser.

```html
<script src="https://ayonli.github.io/jsext/bundle/jsext.js">
    // this will also include the sub-modules and augmentations
</script>
```

### Note for Cloudflare Workers and Fastly Compute

For applications run in Cloudflare Workers and Fastly Compute, install the NPM
version of this package instead of the JSR version.

### Note for TypeScript Project

This package requires `compilerOptions.moduleResolution` set to `Bundler` or
`NodeNext` in `tsconfig.json`.

## Language-enhancing Functions

- [_try](https://jsr.io/@ayonli/jsext/doc/try/~/_try) Calls a function safely and
  return errors when captured.
- [func](https://jsr.io/@ayonli/jsext/doc/func/~/func) Declares a function along with
  a `defer` keyword, inspired by
  Golang.
- [wrap](https://jsr.io/@ayonli/jsext/doc/wrap/~/wrap) Wraps a function for decorator
  pattern but keep its signature.
- [mixin](https://jsr.io/@ayonli/jsext/doc/mixin/~/mixin) Declares a class that
  combines all methods from the base classes.
- [throttle](https://jsr.io/@ayonli/jsext/doc/throttle/~/throttle) Throttles function
  calls for frequent access.
- [debounce](https://jsr.io/@ayonli/jsext/doc/debounce/~/debounce) Debounces function
  calls for frequent access.
- [queue](https://jsr.io/@ayonli/jsext/doc/queue/~/queue) Handles tasks sequentially
  and prevent concurrency conflicts.
- [lock](https://jsr.io/@ayonli/jsext/doc/lock/~/lock) Provides mutual exclusion for
  concurrent operations.
- [chan](https://jsr.io/@ayonli/jsext/doc/chan/~/chan) Creates a channel that
  transfers data across routines, even across multiple threads, inspired by
  Golang.
- [parallel](https://jsr.io/@ayonli/jsext/doc/parallel/~/default) Runs functions in
  parallel threads and take advantage of multi-core CPUs, inspired by Golang.
- [run](https://jsr.io/@ayonli/jsext/doc/run/~/default) Runs a script in another thread
  and abort at any time.
- [deprecate](https://jsr.io/@ayonli/jsext/doc/deprecate/~/deprecate) Marks a function as
  deprecated and emit warnings when it is called.
- [pipe](https://jsr.io/@ayonli/jsext/doc/pipe/~/pipe) Performs pipe operations
  through a series of functions upon a value.

## Subcategories

Each of these modules includes specific functions and classes for their target
categories:

- [archive](https://jsr.io/@ayonli/jsext/doc/archive/~) (Experimental) Collecting
  files into an archive file, or extracting files from a archive file.
- [array](https://jsr.io/@ayonli/jsext/doc/array/~) Functions for dealing with
  arrays.
- [async](https://jsr.io/@ayonli/jsext/doc/async/~) Functions for async/promise
  context handling.
- [bytes](https://jsr.io/@ayonli/jsext/doc/bytes/~) Functions for dealing with
  byte arrays (`Uint8Array`).
- [class](https://jsr.io/@ayonli/jsext/doc/class/~) Functions for dealing with
  classes.
- [cli](https://jsr.io/@ayonli/jsext/doc/cli/~) (Experimental) Useful utility
  functions for interacting with the terminal.
- [collections](https://jsr.io/@ayonli/jsext/doc/collections/~) Additional
  collection data types.
- [dialog](https://jsr.io/@ayonli/jsext/doc/dialog/~) (Experimental)
  Asynchronous dialog functions for both browsers and terminals.
- [encoding](https://jsr.io/@ayonli/jsext/doc/encoding/~) Utilities for encoding
  and decoding binary representations like hex and base64 strings.
- [error](https://jsr.io/@ayonli/jsext/doc/error/~) Functions for converting
  errors to/from other types of objects.
- [event](https://jsr.io/@ayonli/jsext/doc/event/~) Functions for working with
  events.
- [filetype](https://jsr.io/@ayonli/jsext/doc/filetype/~) Functions to get file
  types in different fashions.
- [fs](https://jsr.io/@ayonli/jsext/doc/fs/~) (Experimental) Universal file
  system APIs for both server and browser applications.
- [hash](https://jsr.io/@ayonli/jsext/doc/hash/~) Simplified hash functions for
  various data types.
- [http](https://jsr.io/@ayonli/jsext/doc/http/~) (Experimental) functions for
  handling HTTP related tasks, such as parsing headers and serving HTTP requests.
- [json](https://jsr.io/@ayonli/jsext/doc/json/~) Functions for parsing JSONs to
  specific structures.
- [math](https://jsr.io/@ayonli/jsext/doc/math/~) Functions for mathematical
  calculations.
- [module](https://jsr.io/@ayonli/jsext/doc/module/~) Utility functions for
  working with JavaScript modules.
- [number](https://jsr.io/@ayonli/jsext/doc/number/~) Functions for dealing with
  numbers.
- [object](https://jsr.io/@ayonli/jsext/doc/object/~) Functions for dealing with
  objects.
- [path](https://jsr.io/@ayonli/jsext/doc/path/~) Platform-independent utility
  functions for dealing with file system paths and URLs.
- [reader](https://jsr.io/@ayonli/jsext/doc/reader/~) Utility functions for
  reading data from various types of source into various forms.
- [runtime](https://jsr.io/@ayonli/jsext/doc/runtime/~) (Experimental) Utility
  functions to retrieve runtime information or configure runtime behaviors.
- [sse](https://jsr.io/@ayonli/jsext/doc/sse/~) (Experimental) Tools for
  processing Server-sent Events requests and handling message events.
- [string](https://jsr.io/@ayonli/jsext/doc/string/~) Functions for dealing with
  strings.
- [types](https://jsr.io/@ayonli/jsext/doc/types/~) The missing builtin classes
  of JavaScript and utility types for TypeScript.
- [ws](https://jsr.io/@ayonli/jsext/doc/ws/~) (Experimental) A unified
  WebSocket server interface for Node.js, Deno, Bun and Cloudflare Workers.

## Augmentation

This package supports augmenting some functions to the corresponding built-in
types/namespaces, but they should only be used for application development,
don't use them when developing libraries.

_NOTE: this feature is only available by the NPM package, they don't work by_
_the JSR package._

For more details, please check
[this document](https://github.com/ayonli/jsext/blob/main/augment/README.md).
