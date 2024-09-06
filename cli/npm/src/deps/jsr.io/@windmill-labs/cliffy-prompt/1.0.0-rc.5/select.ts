import type { WidenType } from "./_utils.js";
import { brightBlue, underline } from "../../../@std/fmt/0.225.6/colors.js";
import {
  GenericList,
  type GenericListKeys,
  type GenericListOption,
  type GenericListOptionGroup,
  type GenericListOptionGroupSettings,
  type GenericListOptions,
  type GenericListOptionSettings,
  type GenericListSeparatorOption,
  type GenericListSettings,
  isOption,
  isOptionGroup,
} from "./_generic_list.js";
import { GenericPrompt } from "./_generic_prompt.js";
import { getFiguresByKeys } from "./_figures.js";
import { equal } from "../../../@std/assert/1.0.0-rc.2/equal.js";

/** Select prompt options. */
export interface SelectOptions<TValue>
  extends GenericListOptions<TValue, TValue, TValue> {
  /** Keymap to assign key names to prompt actions. */
  keys?: SelectKeys;
  /** An array of child options. */
  options: Array<
    // | Extract<TValue, string | number>
    | Extract<WidenType<TValue>, string | number>
    | SelectOption<TValue>
    | SelectOptionGroup<TValue>
    | GenericListSeparatorOption
  >;
}

/** Select prompt settings. */
export interface SelectSettings<TValue> extends
  GenericListSettings<
    TValue,
    TValue,
    TValue,
    SelectOptionSettings<TValue>,
    SelectOptionGroupSettings<TValue>
  > {
  keys: SelectKeys;
}

/** Select option options. */
export type SelectOption<TValue> = GenericListOption<TValue>;

/** Select option group options. */
export type SelectOptionGroup<TValue> = GenericListOptionGroup<
  TValue,
  GenericListOption<TValue>
>;

/** Select option settings. */
export type SelectOptionSettings<TValue> = GenericListOptionSettings<TValue>;

/** Select option group settings. */
export type SelectOptionGroupSettings<TValue> = GenericListOptionGroupSettings<
  TValue,
  SelectOptionSettings<TValue>
>;

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
export class Select<TValue> extends GenericList<
  TValue,
  TValue,
  TValue,
  SelectOptionSettings<TValue>,
  SelectOptionGroupSettings<TValue>
> {
  protected readonly settings: SelectSettings<TValue>;
  protected options: Array<
    SelectOptionSettings<TValue> | SelectOptionGroupSettings<TValue>
  >;
  protected listIndex: number;
  protected listOffset: number;

  /** Execute the prompt with provided options. */
  public static prompt<TValue>(
    options: SelectOptions<TValue>,
  ): Promise<WidenType<TValue>> {
    return new this(options).prompt() as Promise<WidenType<TValue>>;
  }

  /**
   * Inject prompt value. If called, the prompt doesn't prompt for an input and
   * returns immediately the injected value. Can be used for unit tests or pre
   * selections.
   *
   * @param value Input value.
   */
  public static inject<TValue>(value: TValue): void {
    GenericPrompt.inject(value);
  }

  constructor(options: SelectOptions<TValue>) {
    super();
    this.settings = this.getDefaultSettings(options);
    this.options = this.settings.options.slice();
    this.listIndex = this.getListIndex(this.settings.default);
    this.listOffset = this.getPageOffset(this.listIndex);
  }

  public getDefaultSettings(
    options: SelectOptions<TValue>,
  ): SelectSettings<TValue> {
    return {
      ...super.getDefaultSettings(options),
      options: this.mapOptions(options, options.options),
    };
  }

  /** Map string option values to options and set option defaults. */
  protected mapOptions(
    promptOptions: SelectOptions<TValue>,
    options: Array<
      | Extract<TValue, string | number>
      | Extract<WidenType<TValue>, string | number>
      | SelectOption<TValue>
      | SelectOptionGroup<TValue>
      | GenericListSeparatorOption
    >,
  ): Array<SelectOptionSettings<TValue> | SelectOptionGroupSettings<TValue>> {
    return options.map((option) =>
      isSelectOptionGroup(option)
        ? this.mapOptionGroup(promptOptions, option)
        : typeof option === "string" || typeof option === "number"
        ? this.mapOption(
          promptOptions,
          { value: option as TValue },
        )
        : this.mapOption(promptOptions, option)
    );
  }

  protected input(): string {
    return underline(brightBlue(this.inputValue));
  }

  protected async submit(): Promise<void> {
    if (
      this.isBackButton(this.selectedOption) ||
      isOptionGroup(this.selectedOption)
    ) {
      const info = isOptionGroup(this.selectedOption)
        ? ` To select a group use ${
          getFiguresByKeys(this.settings.keys.open ?? []).join(", ")
        }.`
        : "";
      this.setErrorMessage(`No option selected.${info}`);
      return;
    }

    await super.submit();
  }

  /** Get value of selected option. */
  protected getValue(): TValue {
    const option = this.options[this.listIndex];
    assertIsOption(option);
    return option.value;
  }

  /**
   * Validate input value.
   * @param value User input value.
   * @return True on success, false or error message on error.
   */
  protected validate(value: TValue): boolean | string {
    return this.options.findIndex((
      option: SelectOptionSettings<TValue> | SelectOptionGroupSettings<TValue>,
    ) => isOption(option) && equal(option.value, value)) !== -1;
  }

  /**
   * Map input value to output value.
   * @param value Input value.
   * @return Output value.
   */
  protected transform(value: TValue): TValue {
    return value;
  }

  /**
   * Format output value.
   * @param value Output value.
   */
  protected format(value: TValue): string {
    return this.settings.format?.(value) ??
      this.getOptionByValue(value)?.name ?? String(value);
  }
}

function assertIsOption<
  TValue,
  TOption extends GenericListOption<TValue>,
>(
  option: TOption | GenericListOptionGroup<TValue, GenericListOption<TValue>>,
): asserts option is TOption {
  if (!isOption(option)) {
    throw new Error("Expected an option but got an option group.");
  }
}

export function isSelectOptionGroup(
  option: unknown,
  // deno-lint-ignore no-explicit-any
): option is SelectOptionGroup<any> {
  return isOptionGroup(option);
}

/**
 * Select options type.
 * @deprecated Use `Array<string | SelectOption | SelectOptionGroup>` instead.
 */
export type SelectValueOptions = Array<
  string | SelectOption<string> | SelectOptionGroup<string>
>;

/**
 * Select option settings type.
 * @deprecated Use `Array<SelectOptionSettings | SelectOptionGroupSettings>` instead.
 */
export type SelectValueSettings = Array<
  SelectOptionSettings<string> | SelectOptionGroupSettings<string>
>;
