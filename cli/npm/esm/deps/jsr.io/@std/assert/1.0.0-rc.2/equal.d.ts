/**
 * Deep equality comparison used in assertions
 *
 * @param c The actual value
 * @param d The expected value
 * @returns `true` if the values are deeply equal, `false` otherwise
 *
 * @example Usage
 * ```ts
 * import { equal } from "@std/assert/equal";
 *
 * equal({ foo: "bar" }, { foo: "bar" }); // Returns `true`
 * equal({ foo: "bar" }, { foo: "baz" }); // Returns `false
 * ```
 */
export declare function equal(c: unknown, d: unknown): boolean;
//# sourceMappingURL=equal.d.ts.map