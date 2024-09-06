/**
 * This module includes functions for reading data from various kinds of streams.
 * @module
 * @deprecated Use `@ayonli/jsext/reader` module instead.
 */

import { readAsArray, toAsyncIterable } from "./reader.ts";

/**
 * @deprecated Use {@link toAsyncIterable} from `@ayonli/jsext/reader` instead.
 */
const read = toAsyncIterable;
export default read;

/**
 * @deprecated Use {@link readAsArray} from `@ayonli/jsext/reader` instead.
 */
export const readAll = readAsArray;
