/**
 * Checks if the given object is an Iterable (implemented `@@iterator`).
 */
export declare function isIterable(obj: any): obj is Iterable<any>;
/**
 * Checks if the given object is an AsyncIterable (implemented `@@asyncIterator`).
 */
export declare function isAsyncIterable(obj: any): obj is AsyncIterable<any>;
/**
 * Checks if the given object is an IteratorLike (implemented `next`).
 */
export declare function isIteratorLike(obj: any): obj is { [x: string | symbol]: any; next: Function; };
/**
 * Checks if the given object is an IterableIterator (implemented both
 * `@@iterator` and `next`).
 */
export declare function isIterableIterator(obj: any): obj is IterableIterator<any>;
/**
 * Checks if the given object is an AsyncIterableIterator (implemented 
 * both `@@asyncIterator` and `next`).
 */
export declare function isAsyncIterableIterator(obj: any): obj is AsyncIterableIterator<any>;
/**
 * Checks if the given object is a Generator.
 */
export declare function isGenerator(obj: any): obj is Generator;
/**
 * Checks if the given object is an AsyncGenerator.
 */
export declare function isAsyncGenerator(obj: any): obj is AsyncGenerator;
