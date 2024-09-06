/**
 * Converts the given `source` into an `AsyncIterable` object if it's not one
 * already, returns `null` if failed.
 */
function asAsyncIterable(source) {
    if (typeof source[Symbol.asyncIterator] === "function") {
        return source;
    }
    else if (typeof source[Symbol.iterator] === "function") {
        return {
            [Symbol.asyncIterator]: async function* () {
                for (const value of source) {
                    yield value;
                }
            },
        };
    }
    else if (typeof ReadableStream === "function"
        && source instanceof ReadableStream) {
        const reader = source.getReader();
        return {
            [Symbol.asyncIterator]: async function* () {
                try {
                    while (true) {
                        const { done, value } = await reader.read();
                        if (done) {
                            break;
                        }
                        yield value;
                    }
                }
                finally {
                    reader.releaseLock();
                }
            },
        };
    }
    return null;
}

export { asAsyncIterable };
//# sourceMappingURL=util.js.map
