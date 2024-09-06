/**
 * Functions for converting errors to/from other types of objects.
 * @module
 */

import { Constructor } from "./types.ts";
import { isPlainObject, omit } from "./object.ts";
import Exception, { type ExceptionOptions } from "./error/Exception.ts";
import { createErrorEvent } from "./event.ts";

export { Exception };
export type { ExceptionOptions };

/**
 * Transforms the error to a plain object.
 * 
 * @example
 * ```ts
 * import { toObject } from "@ayonli/jsext/error";
 * 
 * const err = new Error("Something went wrong.");
 * 
 * const obj = toObject(err);
 * console.log(obj);
 * // {
 * //     "@@type": "Error",
 * //     name: "Error",
 * //     message: "Something went wrong.",
 * //     stack: "Error: Something went wrong.\n    at <anonymous>:1:13"
 * // }
 * ```
 */
export function toObject<T extends Error>(err: T): { [x: string | symbol]: any; } {
    if (!(err instanceof Error) && err["name"] && err["message"]) { // Error-like
        err = fromObject(err, Error) as any;
    }

    const obj = {
        "@@type": err.constructor.name,
        ...omit(err, ["toString", "toJSON", "__callSiteEvals"]),
    } as { [x: string | symbol]: any; };

    if (obj["@@type"] === "AggregateError" && Array.isArray(obj["errors"])) {
        obj["errors"] = (obj["errors"] as unknown[]).map(item => {
            return item instanceof Error ? toObject(item) : item;
        });
    }

    return obj;
}

/**
 * Reverses a plain object to a specific error type.
 * 
 * @example
 * ```ts
 * import { fromObject } from "@ayonli/jsext/error";
 * 
 * const obj = {
 *     "@@type": "Error",
 *     name: "Error",
 *     message: "Something went wrong.",
 *     stack: "Error: Something went wrong.\n    at <anonymous>:1:13"
 * };
 * 
 * const err = fromObject(obj);
 * console.log(err);
 * // Error: Something went wrong.
 * //     at <anonymous>:1:13
 * ```
 */
export function fromObject<T extends { name: "Error"; }>(obj: T): Error;
export function fromObject<T extends { name: "EvalError"; }>(obj: T): EvalError;
export function fromObject<T extends { name: "RangeError"; }>(obj: T): RangeError;
export function fromObject<T extends { name: "ReferenceError"; }>(obj: T): ReferenceError;
export function fromObject<T extends { name: "SyntaxError"; }>(obj: T): SyntaxError;
export function fromObject<T extends { name: "TypeError"; }>(obj: T): TypeError;
export function fromObject<T extends { name: "URIError"; }>(obj: T): URIError;
export function fromObject<T extends { name: "Exception"; }>(obj: T): Exception;
export function fromObject<T extends Error>(obj: { [x: string | symbol]: any; }, ctor?: Constructor<Error>): T | null;
export function fromObject<T extends Error>(
    obj: { [x: string | symbol]: any; },
    ctor: Function | undefined = undefined
): T | null {
    // @ts-ignore
    if (!obj?.name) {
        return null;
    }

    // @ts-ignore
    ctor ||= (globalThis[obj["@@type"] || obj.name] || globalThis[obj.name]) as new (...args: any) => T;

    if (!ctor) {
        if (obj["@@type"] === "Exception") {
            ctor = Exception as unknown as new (...args: any) => T;
        } else {
            ctor = Error as unknown as new (...args: any) => T;
        }
    }

    let err: T;

    if (ctor.name === "DOMException" && typeof DOMException === "function") {
        err = new (ctor as typeof DOMException)(obj["message"] ?? "", obj["name"]) as any;
    } else {
        err = Object.create(ctor.prototype, {
            message: {
                configurable: true,
                enumerable: false,
                writable: true,
                value: obj["message"] ?? "",
            },
        });
    }

    if (err.name !== obj["name"]) {
        Object.defineProperty(err, "name", {
            configurable: true,
            enumerable: false,
            writable: true,
            value: obj["name"],
        });
    }

    if (obj["stack"] !== undefined) {
        Object.defineProperty(err, "stack", {
            configurable: true,
            enumerable: false,
            writable: true,
            value: obj["stack"],
        });
    }

    if (obj["cause"] != undefined) {
        Object.defineProperty(err, "cause", {
            configurable: true,
            enumerable: false,
            writable: true,
            value: obj["cause"],
        });
    }

    const otherKeys = Reflect.ownKeys(obj).filter(key => !([
        "@@type",
        "name",
        "message",
        "stack",
        "cause"
    ] as (string | symbol)[]).includes(key));

    otherKeys.forEach(key => {
        // @ts-ignore
        err[key] ??= obj[key];
    });

    // @ts-ignore
    if (isAggregateError(err) && Array.isArray(err["errors"])) {
        (err as any)["errors"] = ((err as any)["errors"] as unknown[]).map(item => {
            return isPlainObject(item) ? fromObject(item) : item;
        });
    }

    return err;
}

/**
 * Creates an `ErrorEvent` instance based on the given error.
 * 
 * @example
 * ```ts
 * import { toErrorEvent } from "@ayonli/jsext/error";
 * 
 * const err = new Error("Something went wrong.");
 * 
 * const event = toErrorEvent(err);
 * console.log(event);
 * // ErrorEvent {
 * //     error: Error: Something went wrong.
 * //         at <anonymous>:1:13,
 * //     message: "Something went wrong.",
 * //     filename: "",
 * //     lineno: 1,
 * //     colno: 13
 * // }
 * ```
 */
export function toErrorEvent(err: Error, type: string = "error"): ErrorEvent {
    let filename = "";
    let lineno = 0;
    let colno = 0;

    if (err.stack) {
        const lines = err.stack.split("\n").map(line => line.trim());
        let callSite = lines.find(line => line.startsWith("at "));

        if (callSite) {
            callSite = callSite.slice(3);
        } else if (callSite = lines.find(line => line.includes("@") && line.length > 1)) {
            callSite = callSite.slice(callSite.indexOf("@") + 1);
        }

        if (callSite) {
            let start = callSite.lastIndexOf("(");
            let end = 0;

            if (start !== -1) {
                start += 1;
                end = callSite.indexOf(")", start);
                callSite = callSite.slice(start, end);
            }

            const matches = callSite.match(/:(\d+):(\d+)$/);

            if (matches) {
                filename = callSite.slice(0, matches.index);
                lineno = parseInt(matches[1] as string);
                colno = parseInt(matches[2] as string);
            }
        }
    }

    return createErrorEvent(type, {
        error: err,
        message: err.message,
        filename,
        lineno,
        colno,
    });
}

/**
 * Creates an error instance based on the given `ErrorEvent` instance.
 * 
 * @example
 * ```ts
 * import { fromErrorEvent } from "@ayonli/jsext/error";
 * 
 * const event = new ErrorEvent("error", {
 *     message: "Something went wrong.",
 *     filename: "",
 *     lineno: 1,
 *     colno: 13,
 * });
 * 
 * const err = fromErrorEvent(event);
 * console.log(err);
 * // Error: Something went wrong.
 * //     at <anonymous>:1:13
 * ```
 */
export function fromErrorEvent<T extends Error>(event: ErrorEvent): T | null {
    if (event.error instanceof Error) {
        return event.error as T;
    }

    let err: T;
    let shouldPatchStack = false;

    if (event.error
        && typeof event.error === "object"
        && event.error["name"]
        && event.error["message"]
    ) { // Error-like
        err = fromObject(event.error, Error) as T;
        shouldPatchStack = !err.stack;
    } else if (event.message) {
        err = new Error(event.message) as T;
        shouldPatchStack = true;
    } else {
        return null;
    }

    if (shouldPatchStack) {
        let stack = "";

        if (typeof navigator === "object" && navigator.userAgent.includes("Firefox")) {
            if (event.filename) {
                stack = "@" + event.filename;
            } else {
                stack = "@debugger eval code";
            }
        } else if (typeof navigator === "object"
            && navigator.userAgent.includes("Safari")
            && !navigator.userAgent.includes("Chrome") // Chrome likes to pretend it's Safari
        ) {
            if (event.filename) {
                stack = "@" + event.filename;
            } else {
                stack = "global code@";
            }
        } else {
            stack = `${err.name}: ${event.message}\n    at ${event.filename || "<anonymous>"}`;
        }

        if (event.lineno && stack !== "global code@") {
            stack += ":" + event.lineno;
        }

        if (event.colno && stack !== "global code@") {
            stack += ":" + event.colno;
        }

        Object.defineProperty(err, "stack", {
            configurable: true,
            enumerable: false,
            writable: true,
            value: stack,
        });
    }

    return err;
}

/** @inner */
export function isDOMException(value: unknown): value is DOMException {
    return ((typeof DOMException === "function") && (value instanceof DOMException))
        || (value instanceof Error && value.constructor.name === "DOMException"); // Node.js v16-
}

/** @inner */
export function isAggregateError(value: unknown): boolean {
    // @ts-ignore
    return (typeof AggregateError === "function" && value instanceof AggregateError)
        || (value instanceof Error && value.constructor.name === "AggregateError");
}
