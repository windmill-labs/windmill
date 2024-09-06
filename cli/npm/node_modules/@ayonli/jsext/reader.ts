/**
 * Utility functions for reading data from various types of source into various
 * forms.
 * @module
 */
import { concat as concatBytes, text } from "./bytes.ts";
import {
    asAsyncIterable,
    toAsyncIterable,
    resolveByteStream,
    resolveReadableStream,
} from "./reader/util.ts";

export { toAsyncIterable, resolveByteStream };

/**
 * Wraps a source as a `ReadableStream` object that can be used to process streaming data.
 * This function is similar to {@link toAsyncIterable}, except it returns a `ReadableStream`
 * object.
 */
export function toReadableStream<T>(iterable: AsyncIterable<T> | Iterable<T>): ReadableStream<T>;
export function toReadableStream<T>(
    iterable: ReadableStream<T> | Promise<ReadableStream<T>>
): ReadableStream<T>;
export function toReadableStream(es: EventSource, options?: { event?: string; }): ReadableStream<string>;
export function toReadableStream<T extends Uint8Array | string>(ws: WebSocket): ReadableStream<T>;
export function toReadableStream<T>(target: EventTarget, eventMap?: {
    message?: string;
    error?: string;
    close?: string;
}): ReadableStream<T>;
export function toReadableStream<T>(target: NodeJS.EventEmitter, eventMap?: {
    data?: string;
    error?: string;
    close?: string;
}): ReadableStream<T>;
export function toReadableStream<T>(source: any, eventMap: {
    event?: string; // for EventSource custom event
    message?: string;
    data?: string;
    error?: string;
    close?: string;
} | undefined = undefined): ReadableStream<T> {
    if (source instanceof ReadableStream) {
        return source;
    } else if (typeof source["then"] === "function") {
        return resolveReadableStream(source);
    }

    const iterable = toAsyncIterable(source, eventMap) as AsyncIterable<T>;
    const iterator = iterable[Symbol.asyncIterator]();

    return new ReadableStream<T>({
        async pull(controller) {
            const { done, value } = await iterator.next();

            if (done) {
                controller.close();
            } else {
                controller.enqueue(value);
            }
        },
        cancel(reason = undefined) {
            iterator.throw?.(reason);
        }
    });
}

/**
 * Reads all data from the iterable object or readable stream to an array.
 * 
 * @example
 * ```ts
 * import { readAsArray } from "@ayonli/jsext/reader";
 * import * as fs from "node:fs";
 * 
 * const file = fs.createReadStream("./package.json");
 * const chunks = await readAsArray(file);
 * 
 * console.log(chunks);
 * ```
 */
export async function readAsArray<T>(source: AsyncIterable<T> | ReadableStream<T>): Promise<T[]> {
    const iterable = asAsyncIterable(source)!;

    const list: T[] = [];

    for await (const chunk of iterable) {
        list.push(chunk);
    }

    return list;
}

/**
 * Reads all data from the given source to an `ArrayBuffer`.
 * 
 * @example
 * ```ts
 * import { readAsArrayBuffer } from "@ayonli/jsext/reader";
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * 
 * const stream = createReadableStream("./package.json");
 * const buffer = await readAsArrayBuffer(stream);
 * ```
 */
export async function readAsArrayBuffer(
    source: Blob | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>
): Promise<ArrayBuffer> {
    if (typeof Blob === "function" && source instanceof Blob) {
        return await source.arrayBuffer();
    } else if (ArrayBuffer.isView(source)) {
        return source.buffer;
    }

    const iterable = asAsyncIterable(source) as AsyncIterable<Uint8Array> | null;

    if (!iterable) {
        throw new TypeError("The source is not an async iterable object.");
    }

    const chunks = await readAsArray(iterable);
    const bytes = concatBytes(...chunks);

    return bytes.buffer;
}

/**
 * Reads all data from the given source to a `Blob`.
 * 
 * @example
 * ```ts
 * import { readAsBlob } from "@ayonli/jsext/reader";
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * 
 * const stream = createReadableStream("./package.json");
 * const blob = await readAsBlob(stream, "application/json");
 * ```
 */
export async function readAsBlob(
    source: ArrayBuffer | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>,
    type: string
): Promise<Blob> {
    if (source instanceof ArrayBuffer || ArrayBuffer.isView(source)) {
        return new Blob([source], { type });
    }

    const buffer = await readAsArrayBuffer(source);
    return new Blob([buffer], { type });
}

/**
 * Reads all data from the given source to a data URL.
 * 
 * @example
 * ```ts
 * import { readAsDataURL } from "@ayonli/jsext/reader";
 * 
 * const file = new File(["Hello, World!"], "hello.txt", { type: "text/plain" });
 * const dataURL = await readAsDataURL(file, file.type);
 * 
 * console.log(dataURL); // data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==
 * ```
 */
export async function readAsDataURL(
    source: Blob | ArrayBuffer | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>,
    type: string
): Promise<string> {
    if (source instanceof ArrayBuffer) {
        const base64 = text(new Uint8Array(source), "base64");
        return `data:${type};base64,${base64}`;
    } else if (source instanceof Uint8Array) {
        const base64 = text(source, "base64");
        return `data:${type};base64,${base64}`;
    } else if (ArrayBuffer.isView(source)) {
        const bytes = new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
        const base64 = text(bytes, "base64");
        return `data:${type};base64,${base64}`;
    }

    const buffer = await readAsArrayBuffer(source);
    const _bytes = new Uint8Array(buffer);
    const base64 = text(_bytes, "base64");
    return `data:${type};base64,${base64}`;
}

/**
 * Reads all data from the given source to an object URL.
 * 
 * @example
 * ```ts
 * import { readAsObjectURL } from "@ayonli/jsext/reader";
 * 
 * const file = new File(["Hello, World!"], "hello.txt", { type: "text/plain" });
 * const objectURL = await readAsObjectURL(file, file.type);
 * 
 * console.log(objectURL); // e.g. blob:http://localhost:8080/7b8e7b7d-7b7d-7b7d-7b7d-7b7d7b7d7b7d
 * ```
 */
export async function readAsObjectURL(
    source: Blob | ArrayBuffer | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>,
    type: string
): Promise<string> {
    if (source instanceof Blob) {
        return URL.createObjectURL(source);
    } else if (source instanceof ArrayBuffer || ArrayBuffer.isView(source)) {
        const blob = new Blob([source], { type });
        return URL.createObjectURL(blob);
    }

    const buffer = await readAsArrayBuffer(source);
    const blob = new Blob([buffer], { type });
    return URL.createObjectURL(blob);
}

/**
 * Reads all data from the given source to a string.
 * 
 * @example
 * ```ts
 * import { readAsText } from "@ayonli/jsext/reader";
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * 
 * const stream = createReadableStream("./package.json");
 * const text = await readAsText(stream);
 * ```
 */
export async function readAsText(
    source: Blob | ArrayBuffer | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>,
    encoding: string | undefined = undefined
): Promise<string> {
    if (typeof Blob === "function" && source instanceof Blob && !encoding) {
        return await source.text();
    } else if (source instanceof ArrayBuffer || ArrayBuffer.isView(source)) {
        return new TextDecoder(encoding).decode(source);
    }

    const buffer = await readAsArrayBuffer(source);
    return new TextDecoder(encoding).decode(buffer);
}

/**
 * Reads all data from the given source to a JSON object.
 * 
 * @example
 * ```ts
 * import { readAsJSON } from "@ayonli/jsext/reader";
 * import { createReadableStream } from "@ayonli/jsext/fs";
 * 
 * const stream = createReadableStream("./package.json");
 * const pkg = await readAsJSON(stream);
 * ```
 */
export async function readAsJSON<T>(
    source: Blob | ArrayBuffer | ArrayBufferView | AsyncIterable<Uint8Array> | ReadableStream<Uint8Array>,
): Promise<T> {
    const text = await readAsText(source);
    return JSON.parse(text);
}

/**
 * Concatenates multiple readable streams into a single one.
 * 
 * @example
 * ```ts
 * import { concat } from "@ayonli/jsext/read";
 * 
 * const res1 = await fetch("https://example.com/data1");
 * const res2 = await fetch("https://example.com/data2");
 * 
 * const mergedStream = concat(res1.body!, res2.body!);
 * ```
 */
export function concat<T>(...sources: ReadableStream<T>[]): ReadableStream<T>;
/**
 * Concatenates multiple async iterable objects into a single one.
 * 
 * @example
 * ```ts
 * import { concat } from "@ayonli/jsext/read";
 * import { sleep } from "@ayonli/jsext/async";
 * 
 * async function* gen1() {
 *     yield 1;
 *     await sleep(100);
 *     yield 2;
 *     await sleep(100);
 *     yield 3;
 * }
 * 
 * async function* gen2() {
 *     yield 4;
 *     await sleep(100);
 *     yield 5;
 * }
 * 
 * const iter1 = gen1();
 * const iter2 = gen2();
 * 
 * for await (const value of concat(iter1, iter2)) {
 *     console.log(value);
 * }
 * // output:
 * // 1
 * // 2
 * // 3
 * // 4
 * // 5
 * ```
 */
export function concat<T>(...sources: AsyncIterable<T>[]): AsyncIterable<T>;
export function concat<T>(
    ...sources: (AsyncIterable<T>[]) | (ReadableStream<T>[])
): AsyncIterable<T> | ReadableStream<T> {
    if (!sources[0]) {
        throw new TypeError("No sources provided");
    }

    if (typeof ReadableStream === "function" && sources[0] instanceof ReadableStream) {
        if (!sources.every(source => source instanceof ReadableStream)) {
            throw new TypeError("All sources must be readable streams");
        }

        const streams = sources as ReadableStream<T>[];
        let current: number = 0;
        let reader = streams[current]!.getReader();

        return new ReadableStream<T>({
            async pull(controller) {
                try {
                    let { done, value } = await reader!.read();

                    if (!done) {
                        controller.enqueue(value);
                    } else {
                        reader!.releaseLock();
                        current++;

                        if (current < streams.length) {
                            reader = streams[current]!.getReader();
                            return this.pull!(controller);
                        } else {
                            controller.close();
                        }
                    }
                } catch (err) {
                    reader!.releaseLock();
                    controller.error(err);
                }
            }
        });
    } else {
        if (sources.some(source => typeof (source as any)[Symbol.asyncIterator] !== "function")) {
            throw new TypeError("All sources must be async iterable objects");
        }

        const iterables = sources as AsyncIterable<T>[];
        return {
            [Symbol.asyncIterator]: async function* () {
                for (const source of iterables) {
                    yield* source;
                }
            }
        };
    }
}
