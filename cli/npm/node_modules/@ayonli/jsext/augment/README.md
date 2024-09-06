## Augmentations

This package supports augmenting some functions to the corresponding built-in
types/namespaces, but they should only be used for application development,
don't use them when developing libraries.

_NOTE: This feature is only available by the NPM package, they don't work by_
_the JSR package._

```js
// import all
import "@ayonli/jsext/augment";

// import individual category
import "@ayonli/jsext/augment/array";
import "@ayonli/jsext/augment/collections";
import "@ayonli/jsext/augment/error";
import "@ayonli/jsext/augment/json";
import "@ayonli/jsext/augment/math";
import "@ayonli/jsext/augment/number";
import "@ayonli/jsext/augment/object";
import "@ayonli/jsext/augment/promise";
import "@ayonli/jsext/augment/string";
import "@ayonli/jsext/augment/uint8array";
import "@ayonli/jsext/augment/pipe";
import "@ayonli/jsext/augment/types";
```

### Augment Array

- `Array<T>`
  - `prototype`
    - `first(): T | undefined`
    - `last(): T | undefined`
    - `random(remove?: boolean): T | undefined`
    - `count(item: T): number`
    - `equals(another: T[]): boolean`
    - `includesSlice<T>(subset: T[]): boolean`
    - `startsWith<T>(subset: T[]): boolean`
    - `endsWith<T>(subset: T[]): boolean`
    - `split(delimiter: T): T[][]`
    - `chunk(length: number): T[][]`
    - `unique(): T[]`
    - `uniqueBy<K extends string | number | symbol>(fn: (item: T, i: number) => K): T[]`
    - `shuffle(): T[]`
    - `toShuffled(): T[]`
    - `toReversed(): T[]`
    - `toSorted(fn?: ((a: T, b: T) => number) | undefined): T[]`
    - `orderBy(fn: (item: T, i: number) => string | number | bigint, order?: "asc" | "desc"): T[]`
    - `groupBy<K extends string | number | symbol>(fn: (item: T, i: number) => K, type?: ObjectConstructor): Record<K, T[]>`
    - `groupBy<K>(fn: (item: T, i: number) => K, type: MapConstructor): Map<K, T[]>`
    - `keyBy<K extends string | number | symbol>(fn: (item: T, i: number) => K, type?: ObjectConstructor): Record<K, T>`
    - `keyBy<K>(fn: (item: T, i: number) => K, type: MapConstructor): Map<K, T>`
    - `partition(predicate: (item: T, i: number) => boolean): [T[], T[]]`

### Augment Collections

_These types are augmented to the global scope._

- `BiMap<K, V>` (extends `Map<K, V>`) map to each other.
  - `prototype` (additional)
    - `getKey(value: V): K | undefined`
    - `hasValue(value: V): boolean`
    - `deleteValue(value: V): boolean`
- `CiMap<K extends string, V>` (extends `Map<K, any>`)

### Augment Error

- `Error`
  - `toObject<T extends Error>(err: T): { [x: string | symbol]: any; }`
  - `fromObject<T extends Error>(obj: { [x: string | symbol]: any; }, ctor?: Constructor<T>): T`
  - `toErrorEvent(err: Error, type?: string): ErrorEvent`
  - `fromErrorEvent<T extends Error>(event: ErrorEvent): T | null`
  - `prototype`
    - `toJSON(): { [x: string | symbol]: any; }`

- `Exception` (extends `Error`) _This type is augmented to the global scope._
  - `cause?: unknown`
  - `code: number`

### Augment JSON

- `JSON`
  - `parseAs(text: string, type: StringConstructor): string | null`
  - `parseAs(text: string, type: NumberConstructor): number | null`
  - `parseAs(text: string, type: BigIntConstructor): bigint | null`
  - `parseAs(text: string, type: BooleanConstructor): boolean | null`
  - `parseAs<T>(text: string, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null`
  - `as(data: unknown, type: StringConstructor): string | null`
  - `as(data: unknown, type: NumberConstructor): number | null`
  - `as(data: unknown, type: BigIntConstructor): bigint | null`
  - `as(data: unknown, type: BooleanConstructor): boolean | null`
  - `as<T>(data: unknown, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null`
  - `type(ctor: Constructor<any>): PropertyDecorator`

### Augment Math

- `Math`
  - `sum(...values: number[]): number`
  - `avg(...values: number[]): number`
  - `product(...values: number[]): number`
  - `round(value: number, precision: number): number`

### Augment Number

- `Number`
  - `isFloat(value: unknown): boolean`
  - `isNumeric(value: unknown): boolean`
  - `isBetween(value: number, [min, max]: [number, number]): boolean`
  - `random(min: number, max: number): number`
  - `range(min: number, max: number, step?: number): Generator<number, void, unknown>`
  - `serial(loop?: boolean): Generator<number, void, unknown>`

### Augment Object

- `Object`
  - `hasOwn(obj: any, key: string | number | symbol): boolean`
  - `hasOwnMethod(obj: any, method: string | symbol): boolean`
  - `patch<T extends {}, U>(target: T, source: U): T & U`
  - `patch<T extends {}, U, V>(target: T, source1: U, source2: V): T & U & V`
  - `patch<T extends {}, U, V, W>(target: T, source1: U, source2: V, source3: W): T & U & V & W`
  - `patch(target: object, ...sources: any[]): any`
  - `pick<T extends object, U extends keyof T>(obj: T, keys: U[]): Pick<T, U>`
  - `pick<T>(obj: T, keys: (string | symbol)[]): Partial<T>`
  - `omit<T extends object, U extends keyof T>(obj: T, keys: U[]): Omit<T, U>`
  - `omit<T>(obj: T, keys: (string | symbol)[]): Partial<T>`
  - `as(value: unknown, type: StringConstructor): string | null`
  - `as(value: unknown, type: NumberConstructor): number | null`
  - `as(value: unknown, type: BigIntConstructor): bigint | null`
  - `as(value: unknown, type: BooleanConstructor): boolean | null`
  - `as(value: unknown, type: SymbolConstructor): symbol | null`
  - `as<T>(value: unknown, type: Constructor<T>): T | null`
  - `typeOf<T>(value: T): TypeNames | Constructor<T>`
  - `isValid(value: unknown): boolean`
  - `isPlainObject(value: unknown): value is { [x: string | symbol]: any; }`
  - `sanitize<T extends object>(obj: T, deep?: boolean, options?: { removeNulls?: boolean; removeEmptyStrings?: boolean; removeEmptyObjects?: boolean; removeArrayItems?: boolean; }): T`
  - `sortKeys<T extends object>(obj: T, deep?: boolean): T`
  - `flatKeys<T extends object>(obj: T, depth = 1, options?: { flatArrayIndices?: boolean; }): OmitChildrenNodes<T> & Record<string | number | symbol, any>`
  - `filterEntries<T>(obj: Record<string, T>, predicate: (entry: [string, T]) => boolean): Record<string, T>`
  - `mapEntries<T, O>(obj: Record<string, T>, transformer: (entry: [string, T]) => [string, O]): Record<string, O`
  - `partitionEntries<T>(record: Record<string, T>, predicate: (entry: [string, T]) => boolean): [Record<string, T>, Record<string, T>]`
  - `invert<T extends Record<PropertyKey, PropertyKey>>(record: Readonly<T>,): { [P in keyof T as T[P]]: P; }`

### Augment Promise

- `Promise`
  - `abortable<T>(value: PromiseLike<T>, signal: AbortSignal): Promise<T>`
  - `timeout<T>(value: PromiseLike<T>, ms: number): Promise<T>`
  - `after<T>(value: PromiseLike<T>, ms: number): Promise<T>`
  - `sleep(ms: number): Promise<void>`
  - `until<T>(test: () => T | Promise<T>): Promise<T extends false | null | undefined ? never : T>`
  - `select<T>(tasks: ((signal: AbortSignal) => Promise<T>)[]): Promise<T>`

### Augment String

- `String`
  - `compare(str1: string, str2: string): -1 | 0 | 1`
  - `random(length: number, chars?: string): string`
  - `dedent(strings: TemplateStringsArray, ...values: any[]): string`
  - `prototype`
    - `count(sub: string): number`
    - `capitalize(all?: boolean): string`
    - `hyphenate(): string`
    - `bytes(): ByteArray`
    - `chars(): string[]`
    - `words(): string[]`
    - `lines(): string[]`
    - `chunk(length: number): string[]`
    - `truncate(length: number): string`
    - `trim(chars?: string): string`
    - `trimEnd(chars?: string): string`
    - `trimStart(chars?: string): string`
    - `stripEnd(suffix: string): string`
    - `stripStart(prefix: string): string`
    - `dedent(): string`
    - `byteLength(): number`
    - `isAscii(printableOnly?: boolean): boolean`
    - `isEmoji(): boolean`

### Augment Uint8Array

- `Uint8Array`
  - `copy(src: Uint8Array, dest: Uint8Array): number`
  - `concat<T extends Uint8Array>(...arrays: T[]): T`
  - `compare(arr1: Uint8Array, arr2: Uint8Array): -1 | 0 | 1`
  - `prototype`
    - `equals(another: Uint8Array): boolean`
    - `includesSlice(subset: Uint8Array): boolean`
    - `startsWith(subset: Uint8Array): boolean`
    - `endsWith(subset: Uint8Array): boolean`
    - `split(delimiter: number): this[]`
    - `chunk(length: number): this[]`

### Augment Pipe

- `String.prototype.pipe`
- `Number.prototype.pipe`
- `BigInt.prototype.pipe`
- `Boolean.prototype.pipe`
- `Symbol.prototype.pipe`
- `Date.prototype.pipe`
- `RegExp.prototype.pipe`
- `Error.prototype.pipe`
- `Map.prototype.pipe`
- `Set.prototype.pipe`
- `Array.prototype.pipe`
- `TypedArray.prototype.pipe`
- `ArrayBuffer.prototype.pipe`
- `SharedArrayBuffer.prototype.pipe`
- `Blob.prototype.pipe`
- `Event.prototype.pipe`

### Augment global types

- `Constructor<T>`
- `AbstractConstructor<T>`
- `AsyncFunction`
- `GeneratorFunction`
- `AsyncGeneratorFunction`
- `AsyncFunctionConstructor`
- `TypedArray`
- `RealArrayLike<T>`
- `Optional<T, K extends keyof T>`
- `Ensured<T, K extends keyof T>`
- `ValueOf<T>`
