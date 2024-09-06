import { Exception, toErrorEvent, fromObject, toObject, fromErrorEvent } from "../error.ts";
import { Constructor } from "../types.ts";

declare global {
    interface Error {
        toJSON(): { [x: string]: any; };
    }

    interface ErrorConstructor {
        /** Transforms the error to a plain object. */
        toObject<T extends Error>(err: T): { [x: string | symbol]: any; };
        /** Reverses a plain object to a specific error type. */
        fromObject<T extends { name: "Error"; }>(obj: T): Error;
        fromObject<T extends { name: "EvalError"; }>(obj: T): EvalError;
        fromObject<T extends { name: "RangeError"; }>(obj: T): RangeError;
        fromObject<T extends { name: "ReferenceError"; }>(obj: T): ReferenceError;
        fromObject<T extends { name: "SyntaxError"; }>(obj: T): SyntaxError;
        fromObject<T extends { name: "TypeError"; }>(obj: T): TypeError;
        fromObject<T extends { name: "URIError"; }>(obj: T): URIError;
        fromObject<T extends { name: "Exception"; }>(obj: T): Exception;
        fromObject<T extends Error>(obj: { [x: string | symbol]: any; }, ctor?: Constructor<Error>): T | null;

        /** Creates an `ErrorEvent` instance based on the given error. */
        toErrorEvent(err: Error, type?: string): ErrorEvent;
        /** Creates an error instance based on the given `ErrorEvent` instance. */
        fromErrorEvent<T extends Error>(event: ErrorEvent): T | null;
    }

    class Exception extends Error {
        readonly cause?: unknown;
        readonly code: number;
        constructor(message: string, name?: string);
        constructor(message: string, code?: number);
        constructor(message: string, options: { name?: string; cause?: unknown; code?: number; });
    }
}

//@ts-ignore
globalThis["Exception"] = Exception;

Error.toObject = toObject;
Error.fromObject = fromObject;
Error.toErrorEvent = toErrorEvent;
Error.fromErrorEvent = fromErrorEvent;

Error.prototype.toJSON = function toJSON() {
    return toObject(this);
};
