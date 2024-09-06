import type { Command } from "./command.js";
import type { TypeOrTypeHandler } from "./types.js";
import type { ArgumentValue, CompleteHandlerResult, ValuesHandlerResult } from "./types.js";
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
export declare abstract class Type<TValue> {
    abstract parse(type: ArgumentValue): TValue;
    /**
     * Returns values displayed in help text. If no complete method is provided,
     * these values are also used for shell completions.
     */
    values?(cmd: Command, parent?: Command): ValuesHandlerResult;
    /**
     * Returns shell completion values. If no complete method is provided,
     * values from the values method are used.
     */
    complete?(cmd: Command, parent?: Command): CompleteHandlerResult;
}
export declare namespace Type {
    type infer<TType, TDefault = TType> = TType extends TypeOrTypeHandler<infer Value> ? Value : TDefault;
}
//# sourceMappingURL=type.d.ts.map