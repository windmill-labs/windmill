import type { WidenType } from "./_utils.js";
import { GenericList, type GenericListKeys, type GenericListOption, type GenericListOptionGroup, type GenericListOptionGroupSettings, type GenericListOptions, type GenericListOptionSettings, type GenericListSeparatorOption, type GenericListSettings } from "./_generic_list.js";
/** Select prompt options. */
export interface SelectOptions<TValue> extends GenericListOptions<TValue, TValue, TValue> {
    /** Keymap to assign key names to prompt actions. */
    keys?: SelectKeys;
    /** An array of child options. */
    options: Array<Extract<WidenType<TValue>, string | number> | SelectOption<TValue> | SelectOptionGroup<TValue> | GenericListSeparatorOption>;
}
/** Select prompt settings. */
export interface SelectSettings<TValue> extends GenericListSettings<TValue, TValue, TValue, SelectOptionSettings<TValue>, SelectOptionGroupSettings<TValue>> {
    keys: SelectKeys;
}
/** Select option options. */
export type SelectOption<TValue> = GenericListOption<TValue>;
/** Select option group options. */
export type SelectOptionGroup<TValue> = GenericListOptionGroup<TValue, GenericListOption<TValue>>;
/** Select option settings. */
export type SelectOptionSettings<TValue> = GenericListOptionSettings<TValue>;
/** Select option group settings. */
export type SelectOptionGroupSettings<TValue> = GenericListOptionGroupSettings<TValue, SelectOptionSettings<TValue>>;
/** Select prompt keymap. */
export type SelectKeys = GenericListKeys;
/**
 * Select prompt representation.
 *
 * Simple prompt:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const color: string = await Select.prompt({
 *   message: "Pick a color",
 *   options: ["red", "green", "blue"],
 * });
 * ```
 *
 * Mixed option types:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const value: string | number = await Select.prompt<string | number>({
 *   message: "Pick a color",
 *   options: [1, 2, "3", "4"],
 * });
 * ```
 *
 * None primitive option types:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const date: Date = await Select.prompt({
 *   message: "Pick a date",
 *   options: [
 *     {
 *       name: "Date 1",
 *       value: new Date(100000),
 *     },
 *     {
 *       name: "Date 2",
 *       value: new Date(200000),
 *     },
 *     {
 *       name: "Date 3",
 *       value: new Date(300000),
 *     },
 *   ],
 * });
 * ```
 *
 * Grouped options:
 *
 * ```ts
 * import { Select } from "./mod.ts";
 *
 * const value = await Select.prompt({
 *   message: "Select a value",
 *   options: [{
 *     name: "Group 1",
 *     options: ["foo", "bar", "baz"],
 *   }, {
 *     name: "Group 2",
 *     options: ["beep", "boop"],
 *   }],
 * });
 * ```
 */
export declare class Select<TValue> extends GenericList<TValue, TValue, TValue, SelectOptionSettings<TValue>, SelectOptionGroupSettings<TValue>> {
    protected readonly settings: SelectSettings<TValue>;
    protected options: Array<SelectOptionSettings<TValue> | SelectOptionGroupSettings<TValue>>;
    protected listIndex: number;
    protected listOffset: number;
    /** Execute the prompt with provided options. */
    static prompt<TValue>(options: SelectOptions<TValue>): Promise<WidenType<TValue>>;
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject<TValue>(value: TValue): void;
    constructor(options: SelectOptions<TValue>);
    getDefaultSettings(options: SelectOptions<TValue>): SelectSettings<TValue>;
    /** Map string option values to options and set option defaults. */
    protected mapOptions(promptOptions: SelectOptions<TValue>, options: Array<Extract<TValue, string | number> | Extract<WidenType<TValue>, string | number> | SelectOption<TValue> | SelectOptionGroup<TValue> | GenericListSeparatorOption>): Array<SelectOptionSettings<TValue> | SelectOptionGroupSettings<TValue>>;
    protected input(): string;
    protected submit(): Promise<void>;
    /** Get value of selected option. */
    protected getValue(): TValue;
    /**
     * Validate input value.
     * @param value User input value.
     * @return True on success, false or error message on error.
     */
    protected validate(value: TValue): boolean | string;
    /**
     * Map input value to output value.
     * @param value Input value.
     * @return Output value.
     */
    protected transform(value: TValue): TValue;
    /**
     * Format output value.
     * @param value Output value.
     */
    protected format(value: TValue): string;
}
export declare function isSelectOptionGroup(option: unknown): option is SelectOptionGroup<any>;
/**
 * Select options type.
 * @deprecated Use `Array<string | SelectOption | SelectOptionGroup>` instead.
 */
export type SelectValueOptions = Array<string | SelectOption<string> | SelectOptionGroup<string>>;
/**
 * Select option settings type.
 * @deprecated Use `Array<SelectOptionSettings | SelectOptionGroupSettings>` instead.
 */
export type SelectValueSettings = Array<SelectOptionSettings<string> | SelectOptionGroupSettings<string>>;
//# sourceMappingURL=select.d.ts.map