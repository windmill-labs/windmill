import { isBetween, isFloat, isNumeric, random, range, serial, sequence } from "../number.ts";

declare global {
    interface NumberConstructor {
        /** Returns `true` if the given value is a float, `false` otherwise. */
        isFloat(value: unknown): boolean;
        /**
         * Returns `true` if the given value is a numeric value, `false` otherwise. A numeric value
         * is a number, a bigint, or a string that can be converted as a number or bigint.
         * 
         * **NOTE:** `NaN` is not considered numeric.
         * 
         * @param strict Only returns `true` when the value is of type `number`.
         */
        isNumeric(value: unknown, strict?: boolean): boolean;
        /** Return `true` if a number is between the given range (inclusive). */
        isBetween(value: number, [min, max]: [number, number]): boolean;
        /** Returns a random integer ranged from `min` to `max` (inclusive). */
        random(min: number, max: number): number;
        /** Generates a sequence of numbers from `min` to `max` (inclusive). */
        range(min: number, max: number, step?: number): Generator<number, void, unknown>;
        /**
         * Creates a generator that produces sequential numbers from `1` to
         * `Number.MAX_SAFE_INTEGER`, useful for generating unique IDs.
         * 
         * @param loop Repeat the sequence when the end is reached.
         */
        serial(loop?: boolean): Generator<number, void, unknown>;
        /**
         * Creates a generator that produces sequential numbers from `min` to `max` (inclusive).
         * @deprecated use {@link range} and {@link serial} instead.
         */
        sequence(min: number, max: number, step?: number, loop?: boolean): Generator<number, void, unknown>;
    }
}

Number.isFloat = isFloat;
Number.isNumeric = isNumeric;
Number.isBetween = isBetween;
Number.random = random;
Number.range = range;
Number.serial = serial;
Number.sequence = sequence;
