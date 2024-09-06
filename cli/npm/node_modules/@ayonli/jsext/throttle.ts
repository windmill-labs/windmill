/**
 * Throttles function calls for frequent access.
 * @module
 */

type Throttle = {
    for: any;
    expires?: number;
    result?: { value?: any; error?: unknown; };
    pending?: Promise<any> | undefined;
};
const Cache = new Map<any, Throttle>();

/**
 * Options for the {@link throttle} function.
 */
export interface ThrottleOptions {
    duration: number;
    /**
     * Use the throttle strategy `for` the given key, this will keep the
     * result in a global cache, binding new `handler` function for the same
     * key will result in the same result as the previous, unless the
     * duration has passed. This mechanism guarantees that both creating the
     * throttled function in function scopes and overwriting the handler are
     * possible.
     */
    for?: any;
    /**
     * When turned on, respond with the last cache (if available)
     * immediately, even if it has expired, and update the cache in the
     * background.
     */
    noWait?: boolean;
}

/**
 * Creates a throttled function that will only be run once in a certain amount
 * of time.
 * 
 * If a subsequent call happens within the `duration` (in milliseconds), the
 * previous result will be returned and the `handler` function will not be
 * invoked.
 * 
 * @example
 * ```ts
 * import throttle from "@ayonli/jsext/throttle";
 * import { sleep } from "@ayonli/jsext/async";
 * 
 * const fn = throttle((input: string) => input, 1_000);
 * console.log(fn("foo")); // foo
 * console.log(fn("bar")); // foo
 * 
 * await sleep(1_000);
 * console.log(fn("bar")); // bar
 * ```
 */
export default function throttle<I, Fn extends (this: I, ...args: any[]) => any>(
    handler: Fn,
    duration: number
): Fn;
/**
 * @example
 * ```ts
 * import throttle from "@ayonli/jsext/throttle";
 * import { sleep } from "@ayonli/jsext/async";
 * 
 * const out1 = await throttle(() => Promise.resolve("foo"), {
 *     duration: 1_000,
 *     for: "example",
 * })();
 * console.log(out1); // foo
 * 
 * const out2 = await throttle(() => Promise.resolve("bar"), {
 *     duration: 1_000,
 *     for: "example",
 * })();
 * console.log(out2); // foo
 * 
 * await sleep(1_000);
 * const out3 = await throttle(() => Promise.resolve("bar"), {
 *     duration: 1_000,
 *     for: "example",
 * })();
 * console.log(out3); // bar
 * ```
 */
export default function throttle<I, Fn extends (this: I, ...args: any[]) => any>(
    handler: Fn,
    options: ThrottleOptions
): Fn;
export default function throttle(
    handler: (this: any, ...args: any[]) => any,
    options: number | ThrottleOptions
) {
    const key = typeof options === "number" ? null : options.for;
    const duration = typeof options === "number" ? options : options.duration;
    const noWait = typeof options === "number" ? false : !!options?.noWait;

    const handleCall = function (
        this: any,
        cache: Throttle,
        ...args: any[]
    ) {
        if (cache.result && ((cache.pending && noWait) || Date.now() < (cache.expires ?? 0))) {
            if (cache.result.error) {
                throw cache.result.error;
            } else {
                return cache.result.value;
            }
        } else if (cache.pending) {
            return cache.pending;
        }

        try {
            const returns = handler.call(this, ...args);

            if (typeof returns?.then === "function") {
                cache.pending = Promise.resolve(returns).finally(() => {
                    cache.result = { value: cache.pending };
                    cache.pending = undefined;
                    cache.expires = Date.now() + duration;
                });

                if (noWait && cache.result) {
                    if (cache.result.error) {
                        throw cache.result.error;
                    } else {
                        return cache.result.value;
                    }
                } else {
                    return cache.pending;
                }
            } else {
                cache.result = { value: returns };
                cache.expires = Date.now() + duration;
                return returns;
            }
        } catch (error) {
            cache.result = { error };
            cache.expires = Date.now() + duration;
            throw error;
        }
    };

    if (key === null || key === undefined || key === "") {
        const cache: Throttle = { for: null };
        return function (this: any, ...args: any[]) {
            return handleCall.call(this, cache, ...args);
        };
    } else {
        let cache = Cache.get(key);

        if (!cache) {
            cache = { for: key };
            Cache.set(key, cache);
        }

        return function (this: any, ...args: any[]) {
            return handleCall.call(this, cache as Throttle, ...args);
        };
    }
}
