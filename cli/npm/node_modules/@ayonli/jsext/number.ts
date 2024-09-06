/**
 * Functions for dealing with numbers.
 * @module
 */

/** Returns `true` if the given value is a float number, `false` otherwise. */
export function isFloat(value: unknown): boolean {
    return typeof value === "number"
        && !Number.isNaN(value)
        && (!Number.isFinite(value) || (value as number) % 1 !== 0);
}

/**
 * Returns `true` if the given value is a numeric value, `false` otherwise. A numeric value is a 
 * number, a bigint, or a string that can be converted to a number or bigint.
 * 
 * **NOTE:** `NaN` is not considered numeric.
 * 
 * @param strict Only returns `true` when the value is of type `number`.
 * 
 * @example
 * ```ts
 * import { isNumeric } from "@ayonli/jsext/number";
 * 
 * console.log(isNumeric(42)); // true
 * console.log(isNumeric(42n)); // true
 * console.log(isNumeric("42")); // true
 * 
 * console.log(isNumeric(NaN)); // false
 * console.log(isNumeric(42n, true)); // false
 * console.log(isNumeric("42", true)); // false
 * ```
 */
export function isNumeric(value: unknown, strict = false): boolean {
    const type = typeof value;

    if (strict) {
        return type === "number" && !Number.isNaN(value);
    } else if (type === "bigint") {
        return true;
    } else if (type === "number") {
        return !Number.isNaN(value);
    } else if (type === "string" && value) {
        try {
            BigInt(value as string);
            return true;
        } catch {
            return !Number.isNaN(Number(value));
        }
    }

    return false;
}

/**
 * Return `true` if a number is between the given range (inclusive).
 * 
 * This function is the same as `value >= min && value <= max`.
 */
export function isBetween(value: number, [min, max]: [number, number]): boolean {
    return value >= min && value <= max;
}

/**
 * Returns a random integer ranged from `min` to `max` (inclusive).
 * 
 * @example
 * ```ts
 * import { random } from "@ayonli/jsext/number";
 * 
 * console.log(random(1, 5)); // 1, 2, 3, 4, or 5
 * ```
 */
export function random(min: number, max: number): number {
    return min + Math.floor(Math.random() * (max - min + 1));
}

/**
 * Generates a sequence of numbers from `min` to `max` (inclusive).
 * 
 * @example
 * ```ts
 * import { range } from "@ayonli/jsext/number";
 * 
 * for (const i of range(1, 5)) {
 *     console.log(i);
 * }
 * // output:
 * // 1
 * // 2
 * // 3
 * // 4
 * // 5
 * ```
 */
export function range(
    min: number,
    max: number,
    step = 1
): Generator<number, void, unknown> {
    return sequence(min, max, step);
}

/**
 * Creates a generator that produces sequential numbers from `1` to
 * `Number.MAX_SAFE_INTEGER`, useful for generating unique IDs.
 * 
 * @param loop Repeat the sequence when the end is reached.
 * 
 * @example
 * ```ts
 * import { serial } from "@ayonli/jsext/number";
 * 
 * const idGenerator = serial();
 * 
 * console.log(idGenerator.next().value); // 1
 * console.log(idGenerator.next().value); // 2
 * console.log(idGenerator.next().value); // 3
 * ```
 */
export function serial(loop = false): Generator<number, void, unknown> {
    return sequence(1, Number.MAX_SAFE_INTEGER, 1, loop);
}

/**
 * Creates a generator that produces sequential numbers from `min` to `max` (inclusive).
 * @deprecated use {@link range} and {@link serial} instead.
 */
export function* sequence(
    min: number,
    max: number,
    step = 1,
    loop = false
): Generator<number, void, unknown> {
    let id = min;

    while (true) {
        yield id;

        if ((id += step) > max) {
            if (loop) {
                id = min;
            } else {
                break;
            }
        }
    }
}
