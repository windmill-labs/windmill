/**
 * Functions for mathematical calculations.
 * @module
 */

/**
 * Returns the sum value of the given values.
 * 
 * @example
 * ```ts
 * import { sum } from "@ayonli/jsext/math";
 * 
 * console.log(sum(1, 2, 3)); // 6
 * console.log(sum(1, 2, 3, 4, 5)); // 15
 * ```
 */
export function sum(...values: number[]): number {
    return values.reduce((sum, value) => sum + value, 0);
};

/**
 * Returns the average value of the given values.
 * 
 * @example
 * ```ts
 * import { avg } from "@ayonli/jsext/math";
 * 
 * console.log(avg(1, 2, 3)); // 2
 * console.log(avg(1, 2, 3, 4, 5)); // 3
 * ```
 */
export function avg(...values: number[]): number {
    return sum(...values) / values.length;
};

/**
 * Returns a the product value multiplied by the given values.
 * 
 * @example
 * ```ts
 * import { product } from "@ayonli/jsext/math";
 * 
 * console.log(product(1, 2, 3)); // 6
 * console.log(product(1, 2, 3, 4, 5)); // 120
 * ```
 */
export function product(...values: number[]): number {
    return values.slice(1).reduce((sum, value) => sum * value, values[0] ?? 0);
};

const _round = Math.round;

/**
 * Returns the rounded value of the given number.
 * 
 * @example
 * ```ts
 * import { round } from "@ayonli/jsext/math";
 * 
 * console.log(round(1.2345)); // 1
 * console.log(round(1.2345, 2)); // 1.23
 * console.log(round(1.2345, 3)); // 1.235
 * ```
 */
export function round(value: number, precision = 0): number {
    if (precision > 0) {
        const factor = 10 ** precision;
        return _round(value * factor) / factor;
    } else {
        return _round(value);
    }
}
