import { as, parseAs, type } from "../json.ts";

declare global {
    interface JSON {
        /**
         * Converts a JSON string into an object of the given type.
         * 
         * The type can be `String`, `Number`, `BigInt`, `Boolean`, `Date`, `Array`, `Object` or any
         * class constructor (including Error types). If the class has a static
         * `fromJSON(data: any): T` method, it will be invoked to create the instance (unless the
         * parsed `data` is `null`, which will be skipped without further processing).
         * 
         * This function generally does not perform loose conversion between types, for example,
         * `parseAs('"123"', Number)` will not work, it only reverses to the same type before the
         * data are encoded.
         * 
         * However, for compatibility support, there are some exceptions allowed, which are:
         * 
         * - `string` => `Date`
         * - `number` or `string` => `bigint`
         * - `array` => `Buffer` or `TypedArray` (e.g. `Uint8Array`), when the data only contains
         *  integers.
         * - `object` => `Buffer` or `TypedArray` (e.g. `Uint8Array`), if the data are encoded by
         *  `JSON.stringify()`.
         * - customized in `fromJSON()`
         * 
         * If the data cannot be converted to the given type, this function returns `null`.
         */
        parseAs(text: string, type: StringConstructor): string | null;
        parseAs(text: string, type: NumberConstructor): number | null;
        parseAs(text: string, type: BigIntConstructor): bigint | null;
        parseAs(text: string, type: BooleanConstructor): boolean | null;
        parseAs<T>(text: string, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null;
        /**
         * Converts the data into the given type.
         * 
         * This function is primarily used in {@link JSON.parseAs} and shares the same conversion
         * rules, but it can be used in other scenarios too, for example, inside the `fromJSON`
         * function.
         */
        as(data: unknown, type: StringConstructor): string | null;
        as(data: unknown, type: NumberConstructor): number | null;
        as(data: unknown, type: BigIntConstructor): bigint | null;
        as(data: unknown, type: BooleanConstructor): boolean | null;
        as<T>(data: unknown, type: Constructor<T> & { fromJSON?(data: any): T; }): T | null;
        /**
         * A decorator to instruct that the target property in the class is of a specific type.
         * 
         * When parsing JSON via {@link JSON.parseAs}, this property is guaranteed to be of the
         * given type.
         * 
         * **NOTE:** This decorator only supports TypeScript's `experimentalDecorators`.
         * 
         * @example
         * ```ts
         * class Example {
         *     \@JSON.type(Date)
         *     date: Date;
         * }
         * ```
         */
        type(ctor: Constructor<any>): PropertyDecorator;
    }
}

JSON.parseAs = parseAs;
JSON.as = as;
JSON.type = type;
