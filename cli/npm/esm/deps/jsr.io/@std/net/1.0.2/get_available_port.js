// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
var __addDisposableResource = (this && this.__addDisposableResource) || function (env, value, async) {
    if (value !== null && value !== void 0) {
        if (typeof value !== "object" && typeof value !== "function") throw new TypeError("Object expected.");
        var dispose, inner;
        if (async) {
            if (!Symbol.asyncDispose) throw new TypeError("Symbol.asyncDispose is not defined.");
            dispose = value[Symbol.asyncDispose];
        }
        if (dispose === void 0) {
            if (!Symbol.dispose) throw new TypeError("Symbol.dispose is not defined.");
            dispose = value[Symbol.dispose];
            if (async) inner = dispose;
        }
        if (typeof dispose !== "function") throw new TypeError("Object not disposable.");
        if (inner) dispose = function() { try { inner.call(this); } catch (e) { return Promise.reject(e); } };
        env.stack.push({ value: value, dispose: dispose, async: async });
    }
    else if (async) {
        env.stack.push({ async: true });
    }
    return value;
};
var __disposeResources = (this && this.__disposeResources) || (function (SuppressedError) {
    return function (env) {
        function fail(e) {
            env.error = env.hasError ? new SuppressedError(e, env.error, "An error was suppressed during disposal.") : e;
            env.hasError = true;
        }
        function next() {
            while (env.stack.length) {
                var rec = env.stack.pop();
                try {
                    var result = rec.dispose && rec.dispose.call(rec.value);
                    if (rec.async) return Promise.resolve(result).then(next, function(e) { fail(e); return next(); });
                }
                catch (e) {
                    fail(e);
                }
            }
            if (env.hasError) throw env.error;
        }
        return next();
    };
})(typeof SuppressedError === "function" ? SuppressedError : function (error, suppressed, message) {
    var e = new Error(message);
    return e.name = "SuppressedError", e.error = error, e.suppressed = suppressed, e;
});
/** Options for {@linkcode getAvailablePort}. */
import * as dntShim from "../../../../../_dnt.shims.js";
/**
 * Returns an available network port.
 *
 * > [!IMPORTANT]
 * > In most cases, this function is not needed. Do not use it for trivial uses
 * > such as when using {@linkcode Deno.serve} or {@linkcode Deno.listen}
 * > directly. Instead, set the `port` option to `0` to automatically use an
 * > available port, then get the assigned port from the function's return
 * > object (see "Recommended Usage" example).
 *
 * @param options Options for getting an available port.
 * @returns An available network port.
 *
 * @example Recommended Usage
 *
 * Bad:
 * ```ts no-eval no-assert
 * import { getAvailablePort } from "@std/net/get-available-port";
 *
 * const port = getAvailablePort();
 * Deno.serve({ port }, () => new Response("Hello, world!"));
 * ```
 *
 * Good:
 * ```ts no-eval no-assert
 * const { port } = Deno.serve({ port: 0 }, () => new Response("Hello, world!")).addr;
 * ```
 *
 * Good:
 * ```ts no-eval no-assert
 * import { getAvailablePort } from "@std/net/get-available-port";
 *
 * const command = new Deno.Command(Deno.execPath(), {
 *   args: ["test.ts", "--port", getAvailablePort().toString()],
 * });
 * // ...
 * ```
 */
export function getAvailablePort(options) {
    const env_1 = { stack: [], error: void 0, hasError: false };
    try {
        if (options?.preferredPort) {
            try {
                const env_2 = { stack: [], error: void 0, hasError: false };
                try {
                    // Check if the preferred port is available
                    const listener = __addDisposableResource(env_2, dntShim.Deno.listen({ port: options.preferredPort }), false);
                    return listener.addr.port;
                }
                catch (e_1) {
                    env_2.error = e_1;
                    env_2.hasError = true;
                }
                finally {
                    __disposeResources(env_2);
                }
            }
            catch (e) {
                // If the preferred port is not available, fall through and find an available port
                if (!(e instanceof dntShim.Deno.errors.AddrInUse)) {
                    throw e;
                }
            }
        }
        const listener = __addDisposableResource(env_1, dntShim.Deno.listen({ port: 0 }), false);
        return listener.addr.port;
    }
    catch (e_2) {
        env_1.error = e_2;
        env_1.hasError = true;
    }
    finally {
        __disposeResources(env_1);
    }
}
