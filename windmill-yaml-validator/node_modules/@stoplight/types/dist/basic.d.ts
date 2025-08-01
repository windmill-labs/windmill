export { DeepPartial, DeepReadonly } from 'utility-types';
/** Essentials */
export declare type Primitive = string | number | boolean | undefined | null;
/** Dictionaries related */
export declare type Dictionary<T, K extends string | number = string> = {
    [key in K]: T;
};
export declare type DictionaryValues<T> = T extends Dictionary<infer U> ? U : never;
/** Easy create opaque types ie. types that are subset of their original types (ex: positive numbers, uppercased string) */
export declare type Opaque<K, T> = T & {
    __TYPE__: K;
};
export declare type Optional<T> = T | undefined;
