/**
 * This module provides a mechanism to handle tasks sequentially and prevent
 * concurrency conflicts.
 * @module
 */

import chan, { Channel } from "./chan.ts";

export class Queue<T> {
    private channel?: Channel<T>;
    private errorHandler?: (err: unknown) => void;

    constructor(handler: (data: T) => Promise<void>, bufferSize = 0) {
        this.channel = chan(bufferSize);

        (async () => {
            for await (const data of (this.channel as Channel<T>)) {
                try {
                    await handler.call(void 0, data);
                } catch (err) {
                    this.errorHandler?.call(this, err);
                }
            }
        })().catch(err => {
            this.errorHandler?.call(void 0, err);
        });
    }

    push(data: T): Promise<void> {
        return this.channel!.send(data);
    }

    close() {
        this.channel?.close();
    }

    onError(handler: (err: unknown) => void) {
        this.errorHandler = handler;
    }

    [Symbol.dispose]() {
        this.close();
    }
}

/**
 * Processes data sequentially by the given `handler` function and prevents
 * concurrency conflicts, it returns a {@link Queue} instance that we can push
 * data into.
 * 
 * @param bufferSize The maximum capacity of the underlying channel, once
 * reached, the push operation will block until there is new space available.
 * By default, this option is not set and use a non-buffered channel instead.
 * 
 * @example
 * ```ts
 * import queue from "@ayonli/jsext/queue";
 * 
 * const list: string[] = [];
 * const q = queue(async (str: string) => {
 *     await Promise.resolve(null);
 *     list.push(str);
 * });
 * 
 * q.onError(err => {
 *     console.error(err);
 * })
 * 
 * await q.push("foo");
 * await q.push("foo");
 * 
 * console.log(list.length);
 * q.close();
 * // output:
 * // 2
 * ```
 */
export default function queue<T>(handler: (data: T) => Promise<void>, bufferSize = 0): Queue<T> {
    return new Queue(handler, bufferSize);
}
