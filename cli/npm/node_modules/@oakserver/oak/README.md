# oak

[![oak ci](https://github.com/oakserver/oak/workflows/ci/badge.svg)](https://github.com/oakserver/oak)
[![codecov](https://codecov.io/gh/oakserver/oak/branch/main/graph/badge.svg?token=KEKZ52NXGP)](https://codecov.io/gh/oakserver/oak)
[![deno doc](https://doc.deno.land/badge.svg)](https/deno.land/x/oak/mod.ts)

![Custom badge](https://img.shields.io/endpoint?url=https%3A%2F%2Fdeno-visualizer.danopia.net%2Fshields%2Fdep-count%2Fx%2Foak%2Fmod.ts)
![Custom badge](https://img.shields.io/endpoint?url=https%3A%2F%2Fdeno-visualizer.danopia.net%2Fshields%2Fupdates%2Fx%2Foak%2Fmod.ts)
[![Custom badge](https://img.shields.io/endpoint?url=https%3A%2F%2Fdeno-visualizer.danopia.net%2Fshields%2Flatest-version%2Fx%2Foak%2Fmod.ts)](https://doc.deno.land/https/deno.land/x/oak/mod.ts)

A middleware framework for Deno's native HTTP server,
[Deno Deploy](https://deno.com/deploy) and Node.js 16.5 and later. It also
includes a middleware router.

This middleware framework is inspired by [Koa](https://github.com/koajs/koa/)
and middleware router inspired by
[@koa/router](https://github.com/koajs/router/).

This README focuses on the mechanics of the oak APIs and is intended for those
who are familiar with JavaScript middleware frameworks like Express and Koa as
well as a decent understanding of Deno. If you aren't familiar with these,
please check out documentation on
[oakserver.github.io/oak](https://oakserver.github.io/oak/).

Also, check out our [FAQs](https://oakserver.github.io/oak/FAQ) and the
[awesome-oak](https://oakserver.github.io/awesome-oak/) site of community
resources.

> ⚠️ _Warning_ The examples in this README pull from `main` and are designed for
> Deno CLI or Deno Deploy, which may not make sense to do when you are looking
> to actually deploy a workload. You would want to "pin" to a particular version
> which is compatible with the version of Deno you are using and has a fixed set
> of APIs you would expect. `https://deno.land/x/` supports using git tags in
> the URL to direct you at a particular version. So to use version 3.0.0 of oak,
> you would want to import `https://deno.land/x/oak@v3.0.0/mod.ts`.

## Application, middleware, and context

The `Application` class coordinates managing the HTTP server, running
middleware, and handling errors that occur when processing requests. Two of the
methods are generally used: `.use()` and `.listen()`. Middleware is added via
the `.use()` method and the `.listen()` method will start the server and start
processing requests with the registered middleware.

A basic usage, responding to every request with _Hello World!_:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.use((ctx) => {
  ctx.response.body = "Hello World!";
});

await app.listen({ port: 8000 });
```

You would then run this script in Deno like:

```
> deno run --allow-net helloWorld.ts
```

For more information on running code under Deno, or information on how to
install the Deno CLI, check out the [Deno manual](https://deno.land/manual).

The middleware is processed as a stack, where each middleware function can
control the flow of the response. When the middleware is called, it is passed a
context and reference to the "next" method in the stack.

A more complex example:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

// Logger
app.use(async (ctx, next) => {
  await next();
  const rt = ctx.response.headers.get("X-Response-Time");
  console.log(`${ctx.request.method} ${ctx.request.url} - ${rt}`);
});

// Timing
app.use(async (ctx, next) => {
  const start = Date.now();
  await next();
  const ms = Date.now() - start;
  ctx.response.headers.set("X-Response-Time", `${ms}ms`);
});

// Hello World!
app.use((ctx) => {
  ctx.response.body = "Hello World!";
});

await app.listen({ port: 8000 });
```

To provide an HTTPS server, then the `app.listen()` options need to include the
options `.secure` option set to `true` and supply a `.certFile` and a `.keyFile`
options as well.

### `.handle()` method

The `.handle()` method is used to process requests and receive responses without
having the application manage the server aspect. This though is advanced usage
and most users will want to use `.listen()`.

The `.handle()` method accepts up to three arguments. The first being a
[`Request`](https://developer.mozilla.org/en-US/docs/Web/API/Request) argument,
and the second being a `Deno.Conn` argument. The third optional argument is a
flag to indicate if the request was "secure" in the sense it originated from a
TLS connection to the remote client. The method resolved with a
[`Response`](https://developer.mozilla.org/en-US/docs/Web/API/Response) object
or `undefined` if the `ctx.respond === true`.

An example:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.use((ctx) => {
  ctx.response.body = "Hello World!";
});

const listener = Deno.listen({ hostname: "localhost", port: 8000 });

for await (const conn of listener) {
  (async () => {
    const requests = Deno.serveHttp(conn);
    for await (const { request, respondWith } of requests) {
      const response = await app.handle(request, conn);
      if (response) {
        respondWith(response);
      }
    }
  });
}
```

An instance of application has some properties as well:

- `contextState` - Determines the method used to create state for a new context.
  A value of `"clone"` will set the state as a clone of the app state. A value
  of `"prototype"` means the app's state will be used as the prototype of the
  context's state. A value of `"alias"` means that the application's state and
  the context's state will be a reference to the same object. A value of
  `"empty"` will initialize the context's state with an empty object.

- `.jsonBodyReplacer` - An optional replacer function which will be applied to
  JSON bodies when forming a response.

- `.jsonBodyReviver` - An optional reviver function which will be applied when
  reading JSON bodies in a request.

- `.keys`

  Keys to be used when signing and verifying cookies. The value can be set to an
  array of keys, and instance of `KeyStack`, or an object which provides the
  same interface as `KeyStack` (e.g. an instance of
  [keygrip](https://github.com/crypto-utils/keygrip)). If just the keys are
  passed, oak will manage the keys via `KeyStack` which allows easy key rotation
  without requiring re-signing of data values.

- `.proxy`

  This defaults to `false`, but can be set via the `Application` constructor
  options. This is intended to indicate the application is behind a proxy and
  will use `X-Forwarded-Proto`, `X-Forwarded-Host`, and `X-Forwarded-For` when
  processing the request, which should provide more accurate information about
  the request.

- `.state`

  A record of application state, which can be strongly typed by specifying a
  generic argument when constructing an `Application()`, or inferred by passing
  a state object (e.g. `Application({ state })`).

### Context

The context passed to middleware has several properties:

- `.app`

  A reference to the `Application` that is invoking this middleware.

- `.cookies`

  The `Cookies` instance for this context which allows you to read and set
  cookies.

- `.request`

  The `Request` object which contains details about the request.

- `.respond`

  Determines if when middleware finishes processing, the application should send
  the `.response` to the client. If `true` the response will be sent, and if
  `false` the response will not be sent. The default is `true` but certain
  methods, like `.upgrade()` and `.sendEvents()` will set this to `false`.

- `.response`

  The `Response` object which will be used to form the response sent back to the
  requestor.

- `.socket`

  This will be `undefined` if the connection has not been upgraded to a web
  socket. If the connection has been upgraded, the `.socket` interface will be
  set.

- `.state`

  A record of application state, which can be strongly typed by specifying a
  generic argument when constructing an `Application()`, or inferred by passing
  a state object (e.g. `Application({ state })`).

The context passed to middleware has some methods:

- `.assert()`

  Makes an assertion, which if not true, throws an `HTTPError`, which subclass
  is identified by the second argument, with the message being the third
  argument.

- `.send()`

  Stream a file to the requesting client. See [Static content](#static-content)
  below for more information.

- `.sendEvents()`

  Convert the current connection into a server-sent event response and return a
  `ServerSentEventTarget` where messages and events can be streamed to the
  client. This will set `.respond` to `false`.

- `.throw()`

  Throws an `HTTPError`, which subclass is identified by the first argument,
  with the message being passed as the second.

- `.upgrade()`

  Attempt to upgrade the connection to a web socket connection, and return a
  `WebSocket` interface. Previous version of oak, this would be a `Promise`
  resolving with a `std/ws` web socket.

Unlike other middleware frameworks, `context` does not have a significant amount
of aliases. The information about the request is only located in `.request` and
the information about the response is only located in `.response`.

#### Cookies

The `context.cookies` allows access to the values of cookies in the request, and
allows cookies to be set in the response. It automatically secures cookies if
the `.keys` property is set on the application. Because `.cookies` uses the web
crypto APIs to sign and validate cookies, and those APIs work in an asynchronous
way, the cookie APIs work in an asynchronous way. It has several methods:

- `.get(key: string, options?: CookieGetOptions): Promise<string | undefined>`

  Attempts to retrieve the cookie out of the request and returns the value of
  the cookie based on the key. If the applications `.keys` is set, then the
  cookie will be verified against a signed version of the cookie. If the cookie
  is valid, the promise will resolve with the value. If it is invalid, the
  cookie signature will be set to deleted on the response. If the cookie was not
  signed by the current key, it will be resigned and added to the response.

- `.set(key: string, value: string, options?: CookieSetDeleteOptions): Promise<void>`

  Will set a cookie in the response based on the provided key, value and any
  options. If the applications `.keys` is set, then the cookie will be signed
  and the signature added to the response. As the keys are signed
  asynchronously, awaiting the `.set()` method is advised.

#### Request

The `context.request` contains information about the request. It contains
several properties:

- `.hasBody`

  Set to `true` if the request might have a body, or `false` if it does not. It
  does not validate if the body is supported by the built in body parser though.

  **WARNING** this is an unreliable API. In HTTP/2 in many situations you cannot
  determine if a request has a body or not unless you attempt to read the body,
  due to the streaming nature of HTTP/2. As of Deno 1.16.1, for HTTP/1.1, Deno
  also reflects that behavior. The only reliable way to determine if a request
  has a body or not is to attempt to read the body.

  It is best to determine if a body might be meaningful to you with a given
  method, and then attempt to read and process the body if it is meaningful in a
  given context. For example `GET` and `HEAD` should never have a body, but
  methods like `DELETE` and `OPTIONS` _might_ have a body and should be have
  their body conditionally processed if it is meaningful to your application.

- `.headers`

  The headers for the request, an instance of `Headers`.

- `.method`

  A string that represents the HTTP method for the request.

- `.originalRequest`

  The "raw" `NativeServer` request, which is an abstraction over the DOM
  `Request` object. `.originalRequest.request` is the DOM `Request` instance
  that is being processed. Users should generally avoid using these.

- `.secure`

  A shortcut for `.protocol`, returning `true` if HTTPS otherwise `false`.

- `.url`

  An instance of [`URL`](https://developer.mozilla.org/en-US/docs/Web/API/URL)
  which is based on the full URL for the request. This is in place of having
  parts of the URL exposed on the rest of the request object.

And several methods:

- `.accepts(...types: string[])`

  Negotiates the content type supported by the request for the response. If no
  content types are passed, the method returns a prioritized array of accepted
  content types. If content types are passed, the best negotiated content type
  is returned. If no content type match `undefined` is returned.

- `.acceptsEncodings(...encodings: string[])`

  Negotiates the content encoding supported by the request for the response. If
  no encodings are passed, the method returns a prioritized array of accepted
  encodings. If encodings are passed, the best negotiated encoding is returned.
  If no encodings match `undefined` is returned.

- `.acceptsLanguages(...languages: string[])`

  Negotiates the language the client is able to understand. Where a locale
  variant takes preference. If no encodings are passed, the method returns a
  prioritized array of understood languages. If languages are passed, the best
  negotiated language is returned. If no languages match `undefined` is
  returned.

- `.body(options?: BodyOptions)`

  The method returns a representation of the request body. When no options are
  passed, the request headers are used to determine the type of the body, which
  will be parsed and returned. The returned object contains two properties.
  `type` contains the type of `"json"`, `"text"`, `"form"`, `"form-data"`,
  `"bytes"` or `"undefined"`.

  The type of the `value` can be determined by the value of the `type` property:

  | `type`        | `value`                      |
  | ------------- | ---------------------------- |
  | `"bytes"`     | `Promise<Uint8Array>`        |
  | `"form"`      | `Promise<URLSearchParams>`   |
  | `"form-data"` | `FormDataReader`             |
  | `"json"`      | `Promise<unknown>`           |
  | `"reader"`    | `Deno.Reader`                |
  | `"stream"`    | `ReadableStream<Uint8Array>` |
  | `"text"`      | `Promise<string>`            |
  | `"undefined"` | `undefined`                  |

  If there is no body, the `type` of `"undefined"` is returned. If the content
  type of the request is not recognised, then the `type` of `"bytes"` is
  returned.

  You can use the option `type` to specifically request the body to be returned
  in a particular format. If you need access to the Deno HTTP server's body,
  then you can use the `type` of `"reader"` which will return the body object of
  type `"reader"` with a `value` as a `Deno.Reader`:

  ```ts
  import { readAll } from "https://deno.land/x/std/io/util.ts";

  app.use(async (ctx) => {
    const result = ctx.request.body({ type: "reader" });
    result.type; // "reader"
    await readAll(result.value); // a "raw" Uint8Array of the body
  });
  ```

  Another use case for the `type` option is if certain middleware always needs
  the body in a particular format, but wants other middleware to consume it in a
  content type resolved way:

  ```ts
  app.use(async (ctx) => {
    const result = ctx.request.body({ type: "text" });
    const text = await result.value;
    // do some validation of the body as a string
  });

  app.use(async (ctx) => {
    const result = ctx.request.body(); // content type automatically detected
    if (result.type === "json") {
      const value = await result.value; // an object of parsed JSON
    }
  });
  ```

  You can specify the maximum file size in options of the `read` method of the
  `FormDataReader` to filter incoming files which are larger than that. It'll
  return `undefined` if the size exceeds or it'll return the file as
  `Uint8Array` like this:

  ```ts
  app.use(async (ctx) => {
    try {
      const formDataReader = ctx.request.body({ type: "form-data" }).value;
      const formDataBody = await formDataReader.read({ maxSize: 10000000 }); // Max file size to handle
      const files = formDataBody.files; //Return array of files
      if (files) {
        files.map((file) => {
          file.content; // "undefined" or "Uint8Array"
        });
      }
    } catch (error) {
      // Handle error response
    }
  });
  ```

  You can use the option `contentTypes` to set additional media types that when
  present as the content type for the request, the body will be parsed
  accordingly. The options takes possibly five keys: `json`, `form`, `formData`,
  `text`, and `bytes`. For example if you wanted JavaScript sent to the server
  to be parsed as text, you would do something like this:

  ```ts
  app.use(async (ctx) => {
    const result = ctx.request.body({
      contentTypes: {
        text: ["application/javascript"],
      },
    });
    result.type; // "text"
    await result.value; // a string containing the text
  });
  ```

  Because of the nature of how the body is parsed, once the body is requested
  and returned in a particular format, it can't be requested in certain other
  ones, and `.request.body()` will throw if an incompatible type is requested.
  The types `"form-data"`, `"reader"` and `"stream"` are incompatible with each
  other and all other types, while `"json"`, `"form"`, `"bytes"`, `"text"` are
  all compatible with each other. Although, if there are invalid data for that
  type, they may throw if coerced into that type.

  In particular the `contentTypes.bytes` can be used to override default types
  that are supported that you would want the middleware to handle itself. For
  example if you wanted the middleware to parse all text media types itself, you
  would do something like this:

  ```ts
  app.use(async (ctx) => {
    const result = ctx.request.body({
      contentTypes: {
        bytes: ["text"],
      },
    });
    result.type; // "bytes"
    await result.value; // a Uint8Array of all of the bytes read from the request
  });
  ```

  The option `limit` can be used when reading non-stream type bodies, like text,
  JSON, or bytes. By default it is set to 10 Mib, and ensures that malicious
  requests don't cause unexpected behavior in the server. When there is a body,
  but it doesn't supply a content length, or the content length exceeds the
  limit, trying to await the `.value` of the body will throw. To disable the
  feature and read the body anyways, set the `limit` option to `0` (or
  `Infinity`).

#### Response

The `context.response` contains information about the response which will be
sent back to the requestor. It contains several properties:

- `.body`

  The body of the response, which can often be handled by the automatic response
  body handling documented below.

- `.headers`

  A `Headers` instance which contains the headers for the response.

- `.status`

  An HTTP `Status` code that will be sent back with the response. If this is not
  set before responding, oak will default to `200 OK` if there is a `.body`,
  otherwise `404 Not Found`.

- `.type`

  A media type or extension to set the `Content-Type` header for the response.
  For example, you can provide `txt` or `text/plain` to describe the body.

And a method:

- `.redirect(url?: string | URL | REDIRECT_BACK, alt?: string | URL)`

  A method to simplify redirecting the response to another URL. It will set the
  `Location` header to the supplied `url` and the status to `302 Found` (unless
  the status is already a `3XX` status). The use of symbol `REDIRECT_BACK` as
  the `url` indicates that the `Referer` header in the request should be used as
  the direction, with the `alt` being the alternative location if the `Referer`
  is not set. If neither the `alt` nor the `Referer` are set, the redirect will
  occur to `/`. A basic HTML (if the requestor supports it) or a text body will
  be set explaining they are being redirected.

### Automatic response body handling

When the response `Content-Type` is not set in the headers of the `.response`,
oak will automatically try to determine the appropriate `Content-Type`. First it
will look at `.response.type`. If assigned, it will try to resolve the
appropriate media type based on treating the value of `.type` as either the
media type, or resolving the media type based on an extension. For example if
`.type` was set to `"html"`, then the `Content-Type` will be set to
`"text/html"`.

If `.type` is not set with a value, then oak will inspect the value of
`.response.body`. If the value is a `string`, then oak will check to see if the
string looks like HTML, if so, `Content-Type` will be set to `text/html`
otherwise it will be set to `text/plain`. If the value is an object, other than
a `Uint8Array`, a `Deno.Reader`, or `null`, the object will be passed to
`JSON.stringify()` and the `Content-Type` will be set to `application/json`.

If the type of body is a number, bigint or symbol, it will be coerced to a
string and treated as text.

If the value of body is a function, the function will be called with no
arguments. If the return value of the function is promise like, that will be
await, and the resolved value will be processed as above. If the value is not
promise like, it will be processed as above.

### Opening the server

The application method `.listen()` is used to open the server, start listening
for requests, and processing the registered middleware for each request. This
method returns a promise when the server closes.

Once the server is open, before it starts processing requests, the application
will fire a `"listen"` event, which can be listened for via the
`.addEventListener()` method. For example:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.addEventListener("listen", ({ hostname, port, secure }) => {
  console.log(
    `Listening on: ${secure ? "https://" : "http://"}${
      hostname ?? "localhost"
    }:${port}`,
  );
});

// register some middleware

await app.listen({ port: 80 });
```

### Closing the server

If you want to close the application, the application supports the option of an
[abort signal](https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal).
Here is an example of using the signal:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

const controller = new AbortController();
const { signal } = controller;

// Add some middleware using `app.use()`

const listenPromise = app.listen({ port: 8000, signal });

// In order to close the server...
controller.abort();

// Listen will stop listening for requests and the promise will resolve...
await listenPromise;
// and you can do something after the close to shutdown
```

### Error handling

Middleware can be used to handle other errors with middleware. Awaiting other
middleware to execute while trapping errors works. So if you had an error
handling middleware that provides a well managed response to errors would work
like this:

```ts
import {
  Application,
  isHttpError,
  Status,
} from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.use(async (ctx, next) => {
  try {
    await next();
  } catch (err) {
    if (isHttpError(err)) {
      switch (err.status) {
        case Status.NotFound:
          // handle NotFound
          break;
        default:
          // handle other statuses
      }
    } else {
      // rethrow if you can't handle the error
      throw err;
    }
  }
});
```

Uncaught middleware exceptions will be caught by the application. `Application`
extends the global `EventTarget` in Deno, and when uncaught errors occur in the
middleware or sending of responses, an `EventError` will be dispatched to the
application. To listen for these errors, you would add an event handler to the
application instance:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.addEventListener("error", (evt) => {
  // Will log the thrown error to the console.
  console.log(evt.error);
});

app.use((ctx) => {
  // Will throw a 500 on every request.
  ctx.throw(500);
});

await app.listen({ port: 80 });
```

## Router

The `Router` class produces middleware which can be used with an `Application`
to enable routing based on the pathname of the request.

### Basic usage

The following example serves up a _RESTful_ service of a map of books, where
`http://localhost:8000/book/` will return an array of books and
`http://localhost:8000/book/1` would return the book with ID `"1"`:

```ts
import { Application, Router } from "https://deno.land/x/oak/mod.ts";

const books = new Map<string, any>();
books.set("1", {
  id: "1",
  title: "The Hound of the Baskervilles",
  author: "Conan Doyle, Arthur",
});

const router = new Router();
router
  .get("/", (context) => {
    context.response.body = "Hello world!";
  })
  .get("/book", (context) => {
    context.response.body = Array.from(books.values());
  })
  .get("/book/:id", (context) => {
    if (books.has(context?.params?.id)) {
      context.response.body = books.get(context.params.id);
    }
  });

const app = new Application();
app.use(router.routes());
app.use(router.allowedMethods());

await app.listen({ port: 8000 });
```

A route passed is converted to a regular expression using
[path-to-regexp](https://github.com/pillarjs/path-to-regexp), which means
parameters expressed in the pattern will be converted. `path-to-regexp` has
advanced usage which can create complex patterns which can be used for matching.
Check out the
[documentation for that library](https://github.com/pillarjs/path-to-regexp#parameters)
if you have advanced use cases.

In most cases, the type of `context.params` is automatically inferred from the
path template string through typescript magic. In more complex scenarios this
might not yield the correct result however. In that case you can override the
type with `router.get<RouteParams>`, where `RouteParams` is the explicit type
for `context.params`.

### Nested routers

Nesting routers is supported. The following example responds to
`http://localhost:8000/forums/oak/posts` and
`http://localhost:8000/forums/oak/posts/nested-routers`.

```typescript
import { Application, Router } from "https://deno.land/x/oak/mod.ts";

const posts = new Router()
  .get("/", (ctx) => {
    ctx.response.body = `Forum: ${ctx.params.forumId}`;
  })
  .get("/:postId", (ctx) => {
    ctx.response.body =
      `Forum: ${ctx.params.forumId}, Post: ${ctx.params.postId}`;
  });

const forums = new Router().use(
  "/forums/:forumId/posts",
  posts.routes(),
  posts.allowedMethods(),
);

await new Application().use(forums.routes()).listen({ port: 8000 });
```

## Static content

The function `send()` is designed to serve static content as part of a
middleware function. In the most straight forward usage, a root is provided and
requests provided to the function are fulfilled with files from the local file
system relative to the root from the requested path.

A basic usage would look something like this:

```ts
import { Application } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.use(async (context, next) => {
  try {
    await context.send({
      root: `${Deno.cwd()}/examples/static`,
      index: "index.html",
    });
  } catch {
    await next();
  }
});

await app.listen({ port: 8000 });
```

`send()` automatically supports features like providing `ETag` and
`Last-Modified` headers in the response as well as processing `If-None-Match`
and `If-Modified-Since` headers in the request. This means when serving up
static content, clients will be able to rely upon their cached versions of
assets instead of re-downloading them.

### ETag support

The `send()` method automatically supports generating an `ETag` header for
static assets. The header allows the client to determine if it needs to
re-download an asset or not, but it can be useful to calculate `ETag`s for other
scenarios, and oak supplies the `etag` object to provide these functions.

There are two main use cases, first, a middleware function that assesses the
`context.reponse.body` and determines if it can create an `ETag` header for that
body type, and if so sets the `ETag` header on the response. Basic usage would
look something like this:

```ts
import { Application, etag } from "https://deno.land/x/oak/mod.ts";

const app = new Application();

app.use(etag.factory());

// ... other middleware for the application
```

The second use case is lower-level, where you have an entity which you want to
calculate an `ETag` value for, like implementing custom response logic based on
other header information. The `etag.calculate()` method is provided for this,
and it supports calculating `ETag`s for `string`s, `Uint8Array`s, and
`Deno.FileInfo` structures. Basic usage would look something like this:

```ts
import { etag } from "https://deno.land/x/oak/mod.ts";

export async function mw(context, next) {
  await next();
  const value = await etag.calculate("hello deno");
  context.response.headers.set("ETag", value);
}
```

By default, `etag` will calculate weak tags for `Deno.FileInfo` (or
`Deno.FsFile` bodies in the middleware) and strong tags for `string`s and
`Uint8Array`s. This can be changed by passing a `weak` property in the `options`
parameter to either the `factory` or `calculate` methods.

There are also two helper functions which can be used in conjunction with
requests. There is `ifNoneMatch()` and `ifMatch()`. Both take the value of a
header and an entity to compare to.

`ifNoneMatch()` validates if the entities `ETag` doesn't match the supplied
tags, while `ifMatch()` does the opposite. Check out MDN's
[`If-None-Match`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-None-Match)
and
[`If-Match`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/If-Match)
header articles for more information how these headers are used with `ETag`s.

## Helpers

The `mod.ts` also exports a variable named `helpers` which contains functions
that help with managing contexts.

### getQuery(ctx, options?)

The `helpers.getQuery()` function is designed to make it easier to determine
what a request might be querying in the middleware. It takes the supplied
context's `.request.url.searchParams` and converts it to a record object of the
keys and values. For example, it would convert the following request:

```
https://localhost/resource/?foo=bar&baz=qat
```

Into an object like this:

```js
{
  foo: "bar",
  baz: "qat"
}
```

The function can take a couple of options. The `asMap` will result in a `Map`
being returned instead of an object. The `mergeParams` will merge in parameters
that were parsed out of the route. This only works with router contexts, and any
params will be overwritten by the request's search params. If the following URL
was requested:

```
https://localhost/book/1234/page/23?page=32&size=24
```

And the following was the router middleware:

```ts
router.get("/book/:id/page/:page", (ctx) => {
  getQuery(ctx, { mergeParams: true });
});
```

Would result in the return value being:

```js
{
  id: "1234",
  page: "32",
  size: "24"
}
```

## Testing

The `mod.ts` exports an object named `testing` which contains some utilities for
testing oak middleware you might create. See the
[Testing with oak](https://oakserver.github.io/oak/testing) for more
information.

## Node.js

As of oak v10.3, oak is experimentally supported on Node.js 16.5 and later. The
package is available on npm as `@oakserver/oak`. The package exports are the
same as the exports of the `mod.ts` when using under Deno and the package
auto-detects it is running under Node.js.

A basic example using ESM:

**index.mjs**

```js
import { Application } from "@oakserver/oak";

const app = new Application();

app.use((ctx) => {
  ctx.response.body = "Hello from oak under Node.js";
});

app.listen({ port: 8000 });
```

A basic example using CommonJS:

**index.js**

```js
const { Application } = require("@oakserver/oak");

const app = new Application();

app.use((ctx) => {
  ctx.response.body = "Hello from oak under Node.js";
});

app.listen({ port: 8000 });
```

There are a few notes about the support:

- Currently `FormData` bodies do not properly write binary files to disk. This
  will be fixed in future versions.
- Currently only HTTP/1.1 support is available. There are plans to add HTTP/2.
- Web Socket upgrades are not currently supported. This is planned for the
  future. Trying to upgrade to a web socket will cause an error to be thrown.

---

There are several modules that are directly adapted from other modules. They
have preserved their individual licenses and copyrights. All of the modules,
including those directly adapted are licensed under the MIT License.

All additional work is copyright 2018 - 2023 the oak authors. All rights
reserved.
