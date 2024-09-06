/**
 * Options for creating an exception, used by the {@link Exception} constructor.
 */
export interface ExceptionOptions {
    /**
     * The name of the exception, usually use the same names as the
     * `DOMException` instances, such as `TimeoutError`, `NetworkError`, etc.
     */
    name?: string;
    /**
     * The cause of the exception, usually the original error or exception.
     */
    cause?: unknown;
    /**
     * The error code of the exception, usually use the same codes as the HTTP
     * status.
     */
    code?: number;
}

/**
 * A generic exception class, which can be used to represent any kind of error.
 * It's similar to the `DOMException`, but for any JavaScript environment.
 * 
 * @example
 * ```ts
 * // throw an exception with a name
 * import { Exception } from "@ayonli/jsext/error";
 * 
 * throw new Exception("The resource cannot be found", "NotFoundError");
 * ```
 * 
 * @example
 * ```ts
 * // throw an exception with a code
 * import { Exception } from "@ayonli/jsext/error";
 * 
 * throw new Exception("The resource cannot be found", 404);
 * ```
 * 
 * @example
 * ```ts
 * // rethrow an exception with a cause
 * import { Exception } from "@ayonli/jsext/error";
 * 
 * try {
 *     throw new Error("Something went wrong");
 * } catch (error) {
 *     throw new Exception("An error occurred", { cause: error });
 * }
 * ```
 */
export default class Exception extends Error {
    cause?: unknown;
    code: number = 0;

    constructor(message: string, name?: string);
    constructor(message: string, code?: number);
    constructor(message: string, options: ExceptionOptions);
    constructor(message: string, options: number | string | ExceptionOptions = 0) {
        super(message);

        if (typeof options === "number") {
            this.code = options;
        } else if (typeof options === "string") {
            Object.defineProperty(this, "name", {
                configurable: true,
                enumerable: false,
                writable: true,
                value: options,
            });
        } else {
            if (options.name) {
                Object.defineProperty(this, "name", {
                    configurable: true,
                    enumerable: false,
                    writable: true,
                    value: options.name,
                });
            }

            if (options.cause) {
                Object.defineProperty(this, "cause", {
                    configurable: true,
                    enumerable: false,
                    writable: true,
                    value: options.cause,
                });
            }

            if (options.code) {
                this.code = options.code;
            }
        }
    }
}

Object.defineProperty(Exception.prototype, "name", {
    configurable: true,
    enumerable: false,
    writable: true,
    value: "Exception",
});
