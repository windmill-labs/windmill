import { readAsArray } from './reader.js';
import { toAsyncIterable } from './reader/util.js';

/**
 * This module includes functions for reading data from various kinds of streams.
 * @module
 * @deprecated Use `@ayonli/jsext/reader` module instead.
 */
/**
 * @deprecated Use {@link toAsyncIterable} from `@ayonli/jsext/reader` instead.
 */
const read = toAsyncIterable;
/**
 * @deprecated Use {@link readAsArray} from `@ayonli/jsext/reader` instead.
 */
const readAll = readAsArray;

export { read as default, readAll };
//# sourceMappingURL=read.js.map
