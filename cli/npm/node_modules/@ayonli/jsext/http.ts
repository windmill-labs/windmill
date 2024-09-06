/**
 * Functions for handling HTTP related tasks, such as parsing headers and
 * serving HTTP requests.
 * 
 * Many functions in this module are designed to work in all environments, but
 * some of them are only available in server runtimes such as Node.js, Deno,
 * Bun and Cloudflare Workers.
 * 
 * This module itself is a executable script that can be used to serve static
 * files in the current working directory, or we can provide an entry module
 * which has an default export that satisfies the {@link ServeOptions} to start
 * a custom HTTP server.
 * 
 * The script can be run directly with Deno, Bun, or Node.js.
 * 
 * Deno:
 * ```sh
 * deno run --allow-net --allow-read jsr:@ayonli/jsext/http [--port PORT] [DIR]
 * deno run --allow-net --allow-read jsr:@ayonli/jsext/http <entry.ts>
 * ```
 * 
 * Bun:
 * ```sh
 * bun run node_modules/@ayonli/jsext/http.ts [--port PORT] [DIR]
 * bun run node_modules/@ayonli/jsext/http.ts <entry.ts>
 * ```
 * 
 * Node.js (tsx):
 * ```sh
 * tsx node_modules/@ayonli/jsext/http.ts [--port PORT] [DIR]
 * tsx node_modules/@ayonli/jsext/http.ts <entry.ts>
 * ```
 * 
 * In Node.js, we can also do this:
 * 
 * ```sh
 * tsx --import=@ayonli/jsext/http <entry.ts> [--port PORT] [--parallel [NUM]]
 * # or
 * node -r @ayonli/jsext/http <entry.js> [--port PORT] [--parallel [NUM]]
 * ```
 * @module
 * @experimental
 */

import type { Server as HttpServer } from "node:http";
import type { Http2SecureServer } from "node:http2";
import type { Worker } from "node:cluster";
import { asyncTask } from "./async.ts";
import bytes from "./bytes.ts";
import { args, parseArgs } from "./cli.ts";
import { isBun, isDeno, isNode } from "./env.ts";
import { FileInfo, createReadableStream, exists, readDir, readFile, stat } from "./fs.ts";
import { sha256 } from "./hash.ts";
import {
    createRequestContext,
    createTimingFunctions,
    listenFetchEvent,
    renderDirectoryPage,
    withHeaders,
    patchTimingMetrics,
    withWeb as _withWeb,
} from "./http/internal.ts";
import {
    BunServer,
    NetAddress,
    RequestContext,
    RequestHandler,
    RequestErrorHandler,
    ServeOptions,
    ServeStaticOptions,
    Server
} from "./http/server.ts";
import { Range, ifMatch, ifNoneMatch, parseRange } from "./http/util.ts";
import { isMain } from "./module.ts";
import { as } from "./object.ts";
import { extname, join, resolve, startsWith } from "./path.ts";
import { readAsArray } from "./reader.ts";
import { stripStart } from "./string.ts";
import { WebSocketServer } from "./ws.ts";

/**
 * @deprecated This function has been moved to `@ayonli/jsext/http/internal`.
 */
export const withWeb = _withWeb;

export * from "./http/util.ts";
export type {
    NetAddress,
    RequestContext,
    RequestHandler,
    RequestErrorHandler,
    ServeOptions,
    ServeStaticOptions,
    Server,
};

/**
 * Calculates the ETag for a given entity.
 * 
 * @example
 * ```ts
 * import { stat } from "@ayonli/jsext/fs";
 * import { etag } from "@ayonli/jsext/http";
 * 
 * const etag1 = await etag("Hello, World!");
 * 
 * const data = new Uint8Array([1, 2, 3, 4, 5]);
 * const etag2 = await etag(data);
 * 
 * const info = await stat("file.txt");
 * const etag3 = await etag(info);
 * ```
 */
export async function etag(data: string | Uint8Array | FileInfo): Promise<string> {
    if (typeof data === "string" || data instanceof Uint8Array) {
        if (!data.length) {
            // a short circuit for zero length entities
            return `0-47DEQpj8HBSa+/TImW+5JCeuQeR`;
        }

        if (typeof data === "string") {
            data = bytes(data);
        }

        const hash = await sha256(data, "base64");
        return `${data.length.toString(16)}-${hash.slice(0, 27)}`;
    }

    const mtime = data.mtime ?? new Date();
    const hash = await sha256(mtime.toISOString(), "base64");
    return `${data.size.toString(16)}-${hash.slice(0, 27)}`;
}

/**
 * Returns a random port number that is available for listening.
 * 
 * NOTE: This function is not available in the browser and worker runtimes such
 * as Cloudflare Workers.
 * 
 * @param prefer The preferred port number to return if it is available,
 * otherwise a random port is returned.
 * 
 * @param hostname The hostname to bind the port to. Default is "0.0.0.0", only
 * used when `prefer` is set and not `0`.
 */
export async function randomPort(
    prefer: number | undefined = undefined,
    hostname: string | undefined = undefined
): Promise<number> {
    hostname ||= "0.0.0.0";
    if (isDeno) {
        try {
            const listener = Deno.listen({
                hostname,
                port: prefer ?? 0,
            });
            const { port } = listener.addr as Deno.NetAddr;
            listener.close();
            return Promise.resolve(port);
        } catch (err) {
            if (prefer) {
                return randomPort(0);
            } else {
                throw err;
            }
        }
    } else if (isBun) {
        try {
            const listener = Bun.listen({
                hostname,
                port: prefer ?? 0,
                socket: {
                    data: () => { },
                },
            }) as { port: number; stop: (force?: boolean) => void; };
            const { port } = listener;
            listener.stop(true);
            return Promise.resolve(port);
        } catch (err) {
            if (prefer) {
                return randomPort(0);
            } else {
                throw err;
            }
        }
    } else if (isNode) {
        const { createServer, connect } = await import("net");

        if (prefer) {
            // In Node.js listening on a port used by another process may work,
            // so we don't use `listen` method to check if the port is available.
            // Instead, we use the `connect` method to check if the port can be
            // reached, if so, the port is open and we don't use it.
            const isOpen = await new Promise<boolean>((resolve, reject) => {
                const conn = connect(prefer, hostname === "0.0.0.0" ? "localhost" : hostname);
                conn.once("connect", () => {
                    conn.end();
                    resolve(true);
                }).once("error", (err) => {
                    if ((err as any)["code"] === "ECONNREFUSED") {
                        resolve(false);
                    } else {
                        reject(err);
                    }
                });
            });

            if (isOpen) {
                return randomPort(0);
            } else {
                return prefer;
            }
        } else {
            const server = createServer();
            server.listen({ port: 0, exclusive: true });
            const port = (server.address() as any).port as number;

            return new Promise<number>((resolve, reject) => {
                server.close(err => err ? reject(err) : resolve(port));
            });
        }
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Serves HTTP requests with the given options.
 * 
 * This function provides a unified way to serve HTTP requests in all server
 * runtimes, even worker runtimes. It's similar to the `Deno.serve` and
 * `Bun.serve` functions, in fact, it calls them internally when running in the
 * corresponding runtime. When running in Node.js, it uses the built-in `http`
 * or `http2` modules to create the server.
 * 
 * This function also provides easy ways to handle Server-sent Events and
 * WebSockets inside the fetch handler without touching the underlying verbose
 * APIs.
 * 
 * Currently, the following runtimes are supported:
 * 
 * - Node.js (v18.4.1 or above)
 * - Deno
 * - Bun
 * - Cloudflare Workers
 * - Fastly Compute
 * - Service Worker in the browser
 * 
 * NOTE: WebSocket is not supported in Fastly Compute and browser's Service
 * Worker at the moment.
 * 
 * @example
 * ```ts
 * // simple http server
 * import { serve } from "@ayonli/jsext/http";
 * 
 * serve({
 *     fetch(req) {
 *         return new Response("Hello, World!");
 *     },
 * });
 * ```
 * 
 * @example
 * ```ts
 * // set the hostname and port
 * import { serve } from "@ayonli/jsext/http";
 * 
 * serve({
 *     hostname: "localhost",
 *     port: 8787, // same port as Wrangler dev
 *     fetch(req) {
 *         return new Response("Hello, World!");
 *     },
 * });
 * ```
 * 
 * @example
 * ```ts
 * // serve HTTPS/HTTP2 requests
 * import { readFileAsText } from "@ayonli/jsext/fs";
 * import { serve } from "@ayonli/jsext/http";
 * 
 * serve({
 *     key: await readFileAsText("./cert.key"),
 *     cert: await readFileAsText("./cert.pem"),
 *     fetch(req) {
 *         return new Response("Hello, World!");
 *    },
 * });
 * ```
 * 
 * @example
 * ```ts
 * // respond Server-sent Events
 * import { serve } from "@ayonli/jsext/http";
 * 
 * serve({
 *     fetch(req, ctx) {
 *         const { events, response } = ctx.createEventEndpoint();
 *         let count = events.lastEventId ? Number(events.lastEventId) : 0;
 * 
 *         setInterval(() => {
 *             const lastEventId = String(++count);
 *             events.dispatchEvent(new MessageEvent("ping", {
 *                 data: lastEventId,
 *                 lastEventId,
 *             }));
 *         }, 5_000);
 * 
 *         return response;
 *     },
 * });
 * ```
 * 
 * @example
 * ```ts
 * // upgrade to WebSocket
 * import { serve } from "@ayonli/jsext/http";
 * 
 * serve({
 *     fetch(req, ctx) {
 *         const { socket, response } = ctx.upgradeWebSocket();
 * 
 *         socket.addEventListener("message", (event) => {
 *             console.log(event.data);
 *             socket.send("Hello, Client!");
 *         });
 * 
 *         return response;
 *     },
 * });
 * ```
 * 
 * @example
 * ```ts
 * // module mode (for `deno serve`, Bun and Cloudflare Workers)
 * import { serve } from "@ayonli/jsext/http";
 * 
 * export default serve({
 *     type: "module",
 *     fetch(req) {
 *         return new Response("Hello, World!");
 *     },
 * });
 * ```
 */
export function serve(options: ServeOptions): Server {
    const type = isDeno || isBun || isNode ? options.type || "classic" : "classic";
    const ws = new WebSocketServer(options.ws);
    const { fetch: handle, key, cert, onListen, headers } = options;

    const onError: ServeOptions["onError"] = options.onError ?? ((err) => {
        console.error(err);
        return new Response("Internal Server Error", {
            status: 500,
            statusText: "Internal Server Error",
        });
    });

    return new Server(async () => {
        let hostname = options.hostname || "0.0.0.0";
        let port = options.port;
        let controller: AbortController | null = null;
        let server: HttpServer | Http2SecureServer | Deno.HttpServer | BunServer | null = null;

        if (isDeno) {
            if (type === "classic") {
                port ||= await randomPort(8000, hostname);
                controller = new AbortController();
                const task = asyncTask<void>();
                server = Deno.serve({
                    hostname,
                    port,
                    key,
                    cert,
                    signal: controller.signal,
                    onListen: () => task.resolve(),
                }, (req, info) => {
                    const { getTimers, time, timeEnd } = createTimingFunctions();
                    const ctx = createRequestContext(req, {
                        ws,
                        remoteAddress: {
                            family: info.remoteAddr.hostname.includes(":") ? "IPv6" : "IPv4",
                            address: info.remoteAddr.hostname,
                            port: info.remoteAddr.port,
                        },
                        time,
                        timeEnd,
                    });

                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .then(res => patchTimingMetrics(res, getTimers()))
                        .catch(err => _onError(err, req, ctx));
                });
                await task;
            } else {
                hostname = "";
                port = 0;
            }
        } else if (isBun) {
            if (type === "classic") {
                const tls = key && cert ? { key, cert } : undefined;
                port ||= await randomPort(8000, hostname);
                server = Bun.serve({
                    hostname,
                    port,
                    tls,
                    fetch: (req: Request, server: BunServer) => {
                        const { getTimers, time, timeEnd } = createTimingFunctions();
                        const ctx = createRequestContext(req, {
                            ws,
                            remoteAddress: server.requestIP(req)!,
                            time,
                            timeEnd,
                        });

                        const _handle = withHeaders(handle, headers);
                        const _onError = withHeaders(onError, headers);

                        return _handle(req, ctx)
                            .then(res => patchTimingMetrics(res, getTimers()))
                            .catch(err => _onError(err, req, ctx));
                    },
                    websocket: ws.bunListener,
                }) as BunServer;

                ws.bunBind(server);
            } else {
                hostname = "0.0.0.0";
                port = 3000;
            }
        } else if (isNode) {
            if (type === "classic") {
                const reqListener = withWeb((req, info) => {
                    const { getTimers, time, timeEnd } = createTimingFunctions();
                    const ctx = createRequestContext(req, { ws, ...info, time, timeEnd });
                    const _handle = withHeaders(handle, headers);
                    const _onError = withHeaders(onError, headers);

                    return _handle(req, ctx)
                        .then(res => patchTimingMetrics(res, getTimers()))
                        .catch(err => _onError(err, req, ctx));
                });

                if (key && cert) {
                    const { createSecureServer } = await import("node:http2");
                    server = createSecureServer({ key, cert, allowHTTP1: true }, reqListener);
                } else {
                    const { createServer } = await import("node:http");
                    server = createServer(reqListener);
                }

                port ||= await randomPort(8000, hostname);
                await new Promise<void>((resolve) => {
                    if (hostname && hostname !== "0.0.0.0") {
                        (server as HttpServer | Http2SecureServer).listen(port, hostname, resolve);
                    } else {
                        (server as HttpServer | Http2SecureServer).listen(port, resolve);
                    }
                });
            } else {
                hostname = "";
                port = 0;
            }
        } else if (typeof addEventListener === "function") {
            hostname = "";
            port = 0;

            if (type === "classic") {
                listenFetchEvent({ ws, fetch: handle, onError, headers });
            }
        } else {
            throw new Error("Unsupported runtime");
        }

        return { http: server, hostname, port, controller };
    }, { type, fetch: handle, onError, onListen, ws, headers, secure: !!key && !!cert });
}

/**
 * Serves static files from a file system directory or KV namespace (in
 * Cloudflare Workers).
 * 
 * NOTE: In Node.js, this function requires Node.js v18.4.1 or above.
 * 
 * NOTE: In Cloudflare Workers, this function requires setting the
 * `[site].bucket` option in the `wrangler.toml` file.
 * 
 * @example
 * ```ts
 * import { serve, serveStatic } from "@ayonli/jsext/http";
 * 
 * // use `serve()` so this program runs in all environments
 * serve({
 *     async fetch(req: Request, ctx) {
 *         const { pathname } = new URL(req.url);
 * 
 *         if (pathname.startsWith("/assets")) {
 *             return await serveStatic(req, {
 *                 fsDir: "./assets",
 *                 kv: ctx.bindings?.__STATIC_CONTENT,
 *                 urlPrefix: "/assets",
 *             });
 *         }
 * 
 *         return new Response("Hello, World!");
 *     }
 * });
 * ```
 * 
 * @example
 * ```toml
 * # wrangler.toml
 * [site]
 * bucket = "./assets"
 * ```
 */
export async function serveStatic(
    req: Request,
    options: ServeStaticOptions = {}
): Promise<Response> {
    const extraHeaders = options.headers ?? {};
    const dir = options.fsDir ?? ".";
    const prefix = options.urlPrefix ? join(options.urlPrefix) : "";
    const url = new URL(req.url);
    const pathname = decodeURIComponent(url.pathname);

    if (prefix && !startsWith(pathname, prefix)) {
        return new Response("Not Found", {
            status: 404,
            statusText: "Not Found",
            headers: extraHeaders,
        });
    }

    const filename = join(dir, stripStart(pathname.slice(prefix.length), "/"));
    let info: FileInfo;

    try {
        info = await stat(filename);
    } catch (err) {
        if (as(err, Error)?.name === "NotFoundError") {
            return new Response(`Not Found`, {
                status: 404,
                statusText: "Not Found",
                headers: extraHeaders,
            });
        } else if (as(err, Error)?.name === "NotAllowedError") {
            return new Response("Forbidden", {
                status: 403,
                statusText: "Forbidden",
                headers: extraHeaders,
            });
        } else {
            return new Response("Internal Server Error", {
                status: 500,
                statusText: "Internal Server Error",
                headers: extraHeaders,
            });
        }
    }

    if (info.kind === "directory") {
        if (!req.url.endsWith("/")) {
            return Response.redirect(req.url + "/", 301);
        } else {
            if (await exists(join(filename, "index.html"))) {
                return serveStatic(new Request(join(req.url, "index.html"), req), options);
            } else if (await exists(join(filename, "index.htm"))) {
                return serveStatic(new Request(join(req.url, "index.htm"), req), options);
            } else if (options.listDir) {
                const entries = await readAsArray(readDir(filename));
                return renderDirectoryPage(pathname, entries, extraHeaders);
            } else {
                return new Response("Forbidden", {
                    status: 403,
                    statusText: "Forbidden",
                    headers: extraHeaders,
                });
            }
        }
    } else if (info.kind !== "file") {
        return new Response("Forbidden", {
            status: 403,
            statusText: "Forbidden",
            headers: extraHeaders,
        });
    }

    const rangeValue = req.headers.get("Range");
    let range: Range | undefined;

    if (rangeValue && info.size) {
        try {
            range = parseRange(rangeValue);
        } catch {
            return new Response("Invalid Range header", {
                status: 416,
                statusText: "Range Not Satisfiable",
                headers: extraHeaders,
            });
        }
    }

    const mtime = info.mtime ?? new Date();
    const _etag = await etag(info);
    const headers = new Headers({
        ...extraHeaders,
        "Accept-Ranges": "bytes",
        "Last-Modified": mtime.toUTCString(),
        "Etag": _etag,
    });

    const ifModifiedSinceValue = req.headers.get("If-Modified-Since");
    const ifNoneMatchValue = req.headers.get("If-None-Match");
    const ifMatchValue = req.headers.get("If-Match");
    let modified = true;

    if (ifModifiedSinceValue) {
        const date = new Date(ifModifiedSinceValue);
        modified = Math.floor(mtime.valueOf() / 1000) > Math.floor(date.valueOf() / 1000);
    } else if (ifNoneMatchValue) {
        modified = ifNoneMatch(ifNoneMatchValue, _etag);
    }

    if (!modified) {
        return new Response(null, {
            status: 304,
            statusText: "Not Modified",
            headers,
        });
    } else if (ifMatchValue && range && !ifMatch(ifMatchValue, _etag)) {
        return new Response("Precondition Failed", {
            status: 412,
            statusText: "Precondition Failed",
            headers,
        });
    }

    if (/^text\/|^application\/(json|yaml|toml|xml|javascript)$/.test(info.type)) {
        headers.set("Content-Type", info.type + "; charset=utf-8");
    } else {
        headers.set("Content-Type", info.type || "application/octet-stream");
    }

    if (info.atime) {
        headers.set("Date", info.atime.toUTCString());
    }

    if (options.maxAge) {
        headers.set("Cache-Control", `public, max-age=${options.maxAge}`);
    }

    if (range) {
        const { ranges, suffix: suffixLength } = range;
        let start: number;
        let end: number;

        if (ranges.length) {
            ({ start } = ranges[0]!);
            end = Math.min(ranges[0]!.end ?? info.size - 1, info.size - 1);
        } else {
            start = Math.max(info.size - suffixLength!, 0);
            end = info.size - 1;
        }

        const data = await readFile(filename);
        const slice = data.subarray(start, end + 1);

        headers.set("Content-Range", `bytes ${start}-${end}/${info.size}`);
        headers.set("Content-Length", String(end - start + 1));

        return new Response(slice, {
            status: 206,
            statusText: "Partial Content",
            headers,
        });
    } else if (!info.size) {
        headers.set("Content-Length", "0");

        return new Response("", {
            status: 200,
            statusText: "OK",
            headers,
        });
    } else {
        headers.set("Content-Length", String(info.size));

        return new Response(createReadableStream(filename), {
            status: 200,
            statusText: "OK",
            headers,
        });
    }
}

declare const Bun: any;

async function startServer(args: string[]) {
    const options = parseArgs(args, {
        alias: { p: "port" }
    });
    const port = Number.isFinite(options["port"]) ? options["port"] as number : undefined;
    const parallel = options["parallel"] as boolean | number | undefined;
    let config: Partial<ServeOptions> = {};
    let fetch: ServeOptions["fetch"];
    let filename = String(options[0] || ".");
    const ext = extname(filename);

    if (/^\.m?(js|ts)x?/.test(ext)) { // custom entry file
        filename = resolve(filename);
        const mod = await import(filename);

        if (typeof mod.default === "object" && typeof mod.default.fetch === "function") {
            config = mod.default as ServeOptions;
            fetch = config.fetch!;
        } else {
            throw new Error(
                "The entry file must have an `export default { fetch }` statement");
        }
    }

    fetch ||= (req) => serveStatic(req, {
        fsDir: filename,
        listDir: true,
    });

    if (isNode) {
        import("node:os").then(async ({ availableParallelism }) => {
            const { default: cluster } = await import("node:cluster");

            if (cluster.isPrimary && parallel) {
                const _port = port || await randomPort(8000);
                const max = typeof parallel === "number" ? parallel : availableParallelism();
                const workers: (Worker | null)[] = new Array(max).fill(null);
                const forkWorker = (i: number) => {
                    const worker = cluster.fork({
                        HTTP_PORT: String(_port),
                    });
                    workers[i] = worker;

                    worker.once("exit", (code) => {
                        workers[i] = null;

                        if (code) {
                            forkWorker(i);
                        }
                    });
                };

                for (let i = 0; i < max; i++) {
                    forkWorker(i);
                }
            } else if (cluster.isWorker && process.env["HTTP_PORT"]) {
                serve({
                    ...config,
                    fetch,
                    port: Number(process.env["HTTP_PORT"]!),
                    type: "classic",
                });
            } else {
                serve({ ...config, fetch, port, type: "classic" });
            }
        });
    } else {
        serve({ ...config, fetch, port, type: "classic" });
    }
}

if ((isDeno || isBun || isNode) && isMain(import.meta)) {
    startServer(args);
} else if (isNode && process.execArgv.some(arg => arg.endsWith("@ayonli/jsext/http"))) {
    const options = parseArgs(process.execArgv, {
        alias: { r: "require" },
        lists: ["require", "import"],
    });
    const args = process.argv.slice(1);

    if (args.length && (
        (options["require"] as string[] | undefined)?.includes("@ayonli/jsext/http") ||
        (options["import"] as string[] | undefined)?.includes("@ayonli/jsext/http")
    )) {
        startServer(args);
    }
}
