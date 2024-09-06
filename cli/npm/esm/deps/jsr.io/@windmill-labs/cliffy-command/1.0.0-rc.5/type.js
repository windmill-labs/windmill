/**
 * Base class for custom types.
 *
 * **Custom type example:**
 *
 * ```ts
 * import { Type, ArgumentValue } from "./mod.ts";
 *
 * export class ColorType extends Type<string> {
 *   public parse({ label, name, value, type }: ArgumentValue): string {
 *     if (["red", "blue"].includes(value)) {
 *       throw new Error(
 *         `${label} "${name}" must be of type "${type}", but got "${value}".` +
 *         "Valid colors are: red, blue"
 *       );
 *     }
 *     return value;
 *   }
 *
 *   public complete(): string[] {
 *     return ["red", "blue"];
 *   }
 * }
 * ```
 */
export class Type {
}
