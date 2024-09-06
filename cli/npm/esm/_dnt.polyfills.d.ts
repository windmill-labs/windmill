declare global {
    interface Object {
        /**
         * Determines whether an object has a property with the specified name.
         * @param o An object.
         * @param v A property name.
         */
        hasOwn(o: object, v: PropertyKey): boolean;
    }
}
export {};
declare global {
    interface ArrayConstructor {
        fromAsync<T>(iterableOrArrayLike: AsyncIterable<T> | Iterable<T | Promise<T>> | ArrayLike<T | Promise<T>>): Promise<T[]>;
        fromAsync<T, U>(iterableOrArrayLike: AsyncIterable<T> | Iterable<T> | ArrayLike<T>, mapFn: (value: Awaited<T>) => U, thisArg?: any): Promise<Awaited<U>[]>;
    }
}
export {};
//# sourceMappingURL=_dnt.polyfills.d.ts.map