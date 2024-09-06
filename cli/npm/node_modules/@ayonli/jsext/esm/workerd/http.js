import bytes from '../bytes.js';
import { as } from '../object.js';
import Exception from '../error/Exception.js';
import '../external/event-target-polyfill/index.js';
import { getMIME } from '../filetype.js';
import { exists, readFile, readDir } from './fs.js';
import { sha256 } from '../hash/web.js';
import { renderDirectoryPage } from '../http/internal.js';
export { withWeb } from '../http/internal.js';
import { Server } from '../http/server.js';
import { parseRange, ifNoneMatch, ifMatch } from '../http/util.js';
export { HTTP_METHODS, HTTP_STATUS, getCookie, getCookies, parseAccepts, parseBasicAuth, parseContentType, parseCookie, parseCookies, parseRequest, parseResponse, setCookie, setFilename, stringifyCookie, stringifyCookies, stringifyRequest, stringifyResponse, suggestResponseType, verifyBasicAuth } from '../http/util.js';
import { join, extname } from '../path.js';
import { readAsArray } from '../reader.js';
import { stripStart } from '../string.js';
import { WebSocketServer } from './ws.js';
import runtime from '../runtime.js';
import { startsWith } from '../path/util.js';
export { parseUserAgent } from '../http/user-agent.js';

async function etag(data) {
    var _a;
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
    const mtime = (_a = data.mtime) !== null && _a !== void 0 ? _a : new Date();
    const hash = await sha256(mtime.toISOString(), "base64");
    return `${data.size.toString(16)}-${hash.slice(0, 27)}`;
}
async function randomPort(prefer = undefined) {
    throw new Error("Unsupported runtime");
}
function serve(options) {
    const { identity } = runtime();
    const type = identity === "workerd" ? options.type || "classic" : "classic";
    const ws = new WebSocketServer(options.ws);
    const { fetch, onError, onListen, headers } = options;
    // @ts-ignore
    return new Server(async () => {
        return { http: null, hostname: "", port: 0 };
    }, { type, fetch, onError, onListen, ws, headers });
}
async function serveStatic(req, options = {}) {
    var _a, _b, _c, _d, _e, _f;
    // @ts-ignore
    const kv = (_a = options.kv) !== null && _a !== void 0 ? _a : globalThis["__STATIC_CONTENT"];
    if (!kv) {
        return new Response("Service Unavailable", {
            status: 503,
            statusText: "Service Unavailable",
        });
    }
    const extraHeaders = (_b = options.headers) !== null && _b !== void 0 ? _b : {};
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
    let filename = stripStart(pathname.slice(prefix.length), "/");
    if (filename === "/" || filename === ".") {
        filename = "";
    }
    if (pathname.endsWith("/")) {
        const indexHtml = filename ? join(filename, "index.html") : "index.html";
        const indexHtm = filename ? join(filename, "index.htm") : "index.htm";
        if (await exists(indexHtml, { root: kv })) {
            const data = await readFile(indexHtml, { root: kv });
            return await serveFile(data, {
                filename: indexHtml,
                reqHeaders: req.headers,
                extraHeaders,
                maxAge: (_c = options.maxAge) !== null && _c !== void 0 ? _c : 0,
            });
        }
        else if (await exists(indexHtm, { root: kv })) {
            const data = await readFile(indexHtm, { root: kv });
            return await serveFile(data, {
                filename: indexHtm,
                reqHeaders: req.headers,
                extraHeaders,
                maxAge: (_d = options.maxAge) !== null && _d !== void 0 ? _d : 0,
            });
        }
        else if (options.listDir) {
            const entries = await readAsArray(readDir(filename, { root: kv }));
            return renderDirectoryPage(pathname, entries, extraHeaders);
        }
        else {
            return new Response("Forbidden", {
                status: 403,
                statusText: "Forbidden",
                headers: extraHeaders,
            });
        }
    }
    else if (filename) {
        try {
            const data = await readFile(filename, { root: kv });
            return await serveFile(data, {
                filename,
                reqHeaders: req.headers,
                extraHeaders,
                maxAge: (_e = options.maxAge) !== null && _e !== void 0 ? _e : 0,
            });
        }
        catch (err) {
            if (((_f = as(err, Exception)) === null || _f === void 0 ? void 0 : _f.name) === "NotFoundError") {
                return new Response("Not Found", {
                    status: 404,
                    statusText: "Not Found",
                    headers: extraHeaders,
                });
            }
            else {
                return new Response("Internal Server Error", {
                    status: 500,
                    statusText: "Internal Server Error",
                    headers: extraHeaders,
                });
            }
        }
    }
    else {
        return Response.redirect(req.url + "/", 301);
    }
}
async function serveFile(data, options) {
    var _a, _b;
    const { filename, reqHeaders, extraHeaders } = options;
    const ext = extname(filename);
    const type = (_a = getMIME(ext)) !== null && _a !== void 0 ? _a : "";
    const rangeValue = reqHeaders.get("Range");
    let range;
    if (rangeValue && data.byteLength) {
        try {
            range = parseRange(rangeValue);
        }
        catch (_c) {
            return new Response("Invalid Range header", {
                status: 416,
                statusText: "Range Not Satisfiable",
                headers: extraHeaders,
            });
        }
    }
    const _etag = await etag(data);
    const headers = new Headers({
        ...extraHeaders,
        "Accept-Ranges": "bytes",
        "Etag": _etag,
    });
    const ifNoneMatchValue = reqHeaders.get("If-None-Match");
    const ifMatchValue = reqHeaders.get("If-Match");
    let modified = true;
    if (ifNoneMatchValue) {
        modified = ifNoneMatch(ifNoneMatchValue, _etag);
    }
    if (!modified) {
        return new Response(null, {
            status: 304,
            statusText: "Not Modified",
            headers,
        });
    }
    else if (ifMatchValue && range && !ifMatch(ifMatchValue, _etag)) {
        return new Response("Precondition Failed", {
            status: 412,
            statusText: "Precondition Failed",
            headers,
        });
    }
    if (type) {
        if (/^text\/|^application\/(json|yaml|toml|xml|javascript)$/.test(type)) {
            headers.set("Content-Type", type + "; charset=utf-8");
        }
        else {
            headers.set("Content-Type", type);
        }
    }
    else {
        headers.set("Content-Type", "application/octet-stream");
    }
    if (options.maxAge) {
        headers.set("Cache-Control", `public, max-age=${options.maxAge}`);
    }
    if (range) {
        const { ranges, suffix: suffixLength } = range;
        let start;
        let end;
        if (ranges.length) {
            ({ start } = ranges[0]);
            end = Math.min((_b = ranges[0].end) !== null && _b !== void 0 ? _b : data.byteLength - 1, data.byteLength - 1);
        }
        else {
            start = Math.max(data.byteLength - suffixLength, 0);
            end = data.byteLength - 1;
        }
        const slice = data.subarray(start, end + 1);
        headers.set("Content-Range", `bytes ${start}-${end}/${data.byteLength}`);
        headers.set("Content-Length", String(end - start + 1));
        return new Response(slice, {
            status: 206,
            statusText: "Partial Content",
            headers,
        });
    }
    else if (!data.byteLength) {
        headers.set("Content-Length", "0");
        return new Response("", {
            status: 200,
            statusText: "OK",
            headers,
        });
    }
    else {
        headers.set("Content-Length", String(data.byteLength));
        return new Response(data, {
            status: 200,
            statusText: "OK",
            headers,
        });
    }
}

export { etag, ifMatch, ifNoneMatch, parseRange, randomPort, serve, serveStatic };
//# sourceMappingURL=http.js.map
