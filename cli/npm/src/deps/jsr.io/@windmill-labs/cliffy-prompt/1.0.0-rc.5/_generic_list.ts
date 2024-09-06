import type { KeyCode } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import {
  GenericInput,
  type GenericInputKeys,
  type GenericInputPromptOptions,
  type GenericInputPromptSettings,
} from "./_generic_input.js";
import type { WidenType } from "./_utils.js";
import { bold, brightBlue, dim, stripAnsiCode, yellow } from "../../../@std/fmt/0.225.6/colors.js";
import { levenshteinDistance } from "../../../@std/text/1.0.0-rc.1/levenshtein_distance.js";
import { Figures, getFiguresByKeys } from "./_figures.js";

type UnsupportedInputOptions = "suggestions" | "list";

/** Generic list prompt options. */
export interface GenericListOptions<TValue, TReturnValue, TRawValue>
  extends
    Omit<
      GenericInputPromptOptions<TReturnValue, TRawValue>,
      UnsupportedInputOptions
    > {
  options: Array<
    | Extract<TValue, string | number>
    | Extract<WidenType<TValue>, string | number>
    | GenericListOption<TValue>
    | GenericListOptionGroup<TValue, GenericListOption<TValue>>
    | GenericListSeparatorOption
  >;
  /** Keymap to assign key names to prompt actions. */
  keys?: GenericListKeys;
  /** Change list pointer. Default is `brightBlue("❯")`. */
  listPointer?: string;
  /** Limit max displayed rows per page. */
  maxRows?: number;
  /** Change search label. Default is `brightBlue("🔎")`. */
  searchLabel?: string;
  /** Enable search. */
  search?: boolean;
  /** Display prompt info. */
  info?: boolean;
  /** Limit maximum amount of breadcrumb items. */
  maxBreadcrumbItems?: number;
  /** Change breadcrumb separator. Default is ` › `. */
  breadcrumbSeparator?: string;
  /** Change back pointer. Default is `❮`. */
  backPointer?: string;
  /** Change group pointer. Default is `❯`. */
  groupPointer?: string;
  /** Change group icon. Default is `📁`. */
  groupIcon?: string | boolean;
  /** Change opened group icon. Default is `📂`. */
  groupOpenIcon?: string | boolean;
  /** Format option value. */
  format?: (value: TValue) => string;
}

/** Generic list prompt settings. */
export interface GenericListSettings<
  TValue,
  TReturnValue,
  TRawValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
> extends GenericInputPromptSettings<TReturnValue, TRawValue> {
  options: Array<TOption | TGroup>;
  keys: GenericListKeys;
  listPointer: string;
  maxRows: number;
  searchLabel: string;
  search?: boolean;
  info?: boolean;
  maxBreadcrumbItems: number;
  breadcrumbSeparator: string;
  backPointer: string;
  groupPointer: string;
  groupIcon: string | false;
  groupOpenIcon: string | false;
  format?: (value: TValue) => string;
}

/** Generic list separator option options. */
export interface GenericListSeparatorOption {
  /** The separator label. */
  name: string;
}

/** Generic list option options. */
export interface GenericListOption<TValue> {
  /** The option value. */
  value: TValue;
  /** The option label. */
  name?: string;
  /** Disable option. Disabled options are displayed but cannot be selected. */
  disabled?: boolean;
}

/** Generic list option group options. */
export interface GenericListOptionGroup<
  TValue,
  TOption extends GenericListOption<TValue>,
> {
  /** The option label. */
  name: string;
  /** An array of child options. */
  options: Array<
    | Extract<TValue, string | number>
    | Extract<WidenType<TValue>, string | number>
    | TOption
    | this
    | GenericListSeparatorOption
  >;
  /** Disable option. Disabled options are displayed but cannot be selected. */
  disabled?: boolean;
}

/** Generic list option settings. */
export interface GenericListOptionSettings<TValue>
  extends GenericListOption<TValue> {
  name: string;
  disabled: boolean;
  indentLevel: number;
}

/** Generic list option group settings. */
export interface GenericListOptionGroupSettings<
  TValue,
  TOption extends GenericListOptionSettings<TValue>,
> extends GenericListOptionGroup<TValue, TOption> {
  disabled: boolean;
  indentLevel: number;
  options: Array<TOption | this>;
}

/** GenericList key options. */
export interface GenericListKeys extends GenericInputKeys {
  /** Select next option keymap. Default is `["down", "d", "n", "2"]`. */
  next?: string[];
  /** Select previous option keymap. Default is `["up", "u", "p", "8"]`. */
  previous?: string[];
  /** Select next page keymap. Default is `["pagedown", "right"]`. */
  nextPage?: string[];
  /** Select previous page keymap. Default is `["pageup", "left"]`. */
  previousPage?: string[];
  /** Select next option keymap. Default is `["right", "enter", "return"]`. */
  open?: string[];
  /** Select next option keymap. Default is `["left", "escape", "enter", "return"]`. */
  back?: string[];
}

interface MatchedOption<
  TValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
> {
  option: TOption | TGroup;
  distance: number;
  children: Array<MatchedOption<TValue, TOption, TGroup>>;
}

/** Generic list prompt representation. */
export abstract class GenericList<
  TValue,
  TReturnValue,
  TRawValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
> extends GenericInput<TReturnValue, TRawValue> {
  protected abstract readonly settings: GenericListSettings<
    TValue,
    TReturnValue,
    TRawValue,
    TOption,
    TGroup
  >;
  protected abstract options: Array<TOption | TGroup>;
  protected abstract listIndex: number;
  protected abstract listOffset: number;
  protected parentOptions: Array<TGroup> = [];

  protected get selectedOption(): TOption | TGroup | undefined {
    return this.options.at(this.listIndex);
  }

  /**
   * Create list separator.
   *
   * @param label Separator label.
   */
  public static separator(label = "------------"): GenericListSeparatorOption {
    return { name: label };
  }

  public getDefaultSettings(
    {
      groupIcon = true,
      groupOpenIcon = groupIcon,
      ...options
    }: GenericListOptions<TValue, TReturnValue, TRawValue>,
  ): GenericListSettings<TValue, TReturnValue, TRawValue, TOption, TGroup> {
    const settings = super.getDefaultSettings(options);
    return {
      ...settings,
      listPointer: options.listPointer ?? brightBlue(Figures.POINTER),
      searchLabel: options.searchLabel ?? brightBlue(Figures.SEARCH),
      backPointer: options.backPointer ?? brightBlue(Figures.POINTER_LEFT),
      groupPointer: options.groupPointer ?? options.listPointer ??
        brightBlue(Figures.POINTER),
      groupIcon: !groupIcon
        ? false
        : typeof groupIcon === "string"
        ? groupIcon
        : Figures.FOLDER,
      groupOpenIcon: !groupOpenIcon
        ? false
        : typeof groupOpenIcon === "string"
        ? groupOpenIcon
        : Figures.FOLDER_OPEN,
      maxBreadcrumbItems: options.maxBreadcrumbItems ?? 5,
      breadcrumbSeparator: options.breadcrumbSeparator ??
        ` ${Figures.POINTER_SMALL} `,
      maxRows: options.maxRows ?? 10,
      options: this.mapOptions(options, options.options),
      keys: {
        next: options.search ? ["down"] : ["down", "d", "n", "2"],
        previous: options.search ? ["up"] : ["up", "u", "p", "8"],
        nextPage: ["pagedown", "right"],
        previousPage: ["pageup", "left"],
        open: ["right", "enter", "return"],
        back: ["left", "escape", "enter", "return"],
        ...(settings.keys ?? {}),
      },
    };
  }

  protected abstract mapOptions(
    promptOptions: GenericListOptions<TValue, TReturnValue, TRawValue>,
    options: Array<
      | Extract<TValue, string | number>
      | Extract<WidenType<TValue>, string | number>
      | GenericListOption<TValue>
      | GenericListOptionGroup<TValue, GenericListOption<TValue>>
      | GenericListSeparatorOption
    >,
  ): Array<TOption | TGroup>;

  protected mapOption(
    options: GenericListOptions<TValue, TReturnValue, TRawValue>,
    option: GenericListOption<TValue> | GenericListSeparatorOption,
  ): GenericListOptionSettings<TValue> {
    if (isOption(option)) {
      return {
        value: option.value,
        name: typeof option.name === "undefined"
          ? options.format?.(option.value) ?? String(option.value)
          : option.name,
        disabled: "disabled" in option && option.disabled === true,
        indentLevel: 0,
      };
    } else {
      return {
        value: null as TValue,
        name: option.name,
        disabled: true,
        indentLevel: 0,
      };
    }
  }

  protected mapOptionGroup(
    options: GenericListOptions<TValue, TReturnValue, TRawValue>,
    option: GenericListOptionGroup<TValue, GenericListOption<TValue>>,
    recursive = true,
  ): GenericListOptionGroupSettings<TValue, GenericListOptionSettings<TValue>> {
    return {
      name: option.name,
      disabled: !!option.disabled,
      indentLevel: 0,
      options: recursive ? this.mapOptions(options, option.options) : [],
    };
  }

  protected match(): void {
    const input: string = this.getCurrentInputValue().toLowerCase();
    let options: Array<TOption | TGroup> = this.getCurrentOptions().slice();

    if (input.length) {
      const matches = matchOptions<TValue, TOption, TGroup>(
        input,
        this.getCurrentOptions(),
      );
      options = flatMatchedOptions(matches);
    }

    this.setOptions(options);
  }

  protected setOptions(options: Array<TOption | TGroup>) {
    this.options = [...options];

    const parent = this.getParentOption();
    if (parent && this.options[0] !== parent) {
      this.options.unshift(parent);
    }

    this.listIndex = Math.max(
      0,
      Math.min(this.options.length - 1, this.listIndex),
    );
    this.listOffset = Math.max(
      0,
      Math.min(
        this.options.length - this.getListHeight(),
        this.listOffset,
      ),
    );
  }

  protected getCurrentOptions(): Array<TOption | TGroup> {
    return this.getParentOption()?.options ?? this.settings.options;
  }

  protected getParentOption(index = -1): TGroup | undefined {
    return this.parentOptions.at(index);
  }

  protected submitBackButton() {
    const parentOption = this.parentOptions.pop();
    if (!parentOption) {
      return;
    }
    this.match();
    this.listIndex = this.options.indexOf(parentOption);
  }

  protected submitGroupOption(selectedOption: TGroup) {
    this.parentOptions.push(selectedOption);
    this.match();
    this.listIndex = 0;
  }

  protected isBackButton(option: TOption | TGroup | undefined): boolean {
    return option === this.getParentOption();
  }

  protected hasParent(): boolean {
    return this.parentOptions.length > 0;
  }

  protected isSearching(): boolean {
    return this.getCurrentInputValue() !== "";
  }

  protected message(): string {
    let message = `${this.settings.indent}${this.settings.prefix}` +
      bold(this.settings.message) +
      this.defaults();

    if (this.settings.search) {
      const input = this.isSearchSelected() ? this.input() : dim(this.input());
      message += " " + this.settings.searchLabel + " ";
      this.cursor.x = stripAnsiCode(message).length + this.inputIndex + 1;
      message += input;
    }

    return message;
  }

  /** Render options. */
  protected body(): string | Promise<string> {
    return this.getList() + this.getInfo();
  }

  protected getInfo(): string {
    if (!this.settings.info) {
      return "";
    }
    const selected: number = this.listIndex + 1;
    const hasGroups = this.options.some((option) => isOptionGroup(option));

    const groupActions: Array<[string, Array<string>]> = hasGroups
      ? [
        ["Open", getFiguresByKeys(this.settings.keys.open ?? [])],
        ["Back", getFiguresByKeys(this.settings.keys.back ?? [])],
      ]
      : [];

    const actions: Array<[string, Array<string>]> = [
      ["Next", getFiguresByKeys(this.settings.keys.next ?? [])],
      ["Previous", getFiguresByKeys(this.settings.keys.previous ?? [])],
      ...groupActions,
      ["Next Page", getFiguresByKeys(this.settings.keys.nextPage ?? [])],
      [
        "Previous Page",
        getFiguresByKeys(this.settings.keys.previousPage ?? []),
      ],
      ["Submit", getFiguresByKeys(this.settings.keys.submit ?? [])],
    ];

    return "\n" + this.settings.indent + brightBlue(Figures.INFO) +
      bold(` ${selected}/${this.options.length} `) +
      actions
        .map((cur) => `${cur[0]}: ${bold(cur[1].join(", "))}`)
        .join(", ");
  }

  /** Render options list. */
  protected getList(): string {
    const list: Array<string> = [];
    const height: number = this.getListHeight();
    for (let i = this.listOffset; i < this.listOffset + height; i++) {
      list.push(
        this.getListItem(
          this.options[i],
          this.listIndex === i,
        ),
      );
    }
    if (!list.length) {
      list.push(
        this.settings.indent + dim("  No matches..."),
      );
    }
    return list.join("\n");
  }

  /**
   * Render option.
   * @param option        Option.
   * @param isSelected  Set to true if option is selected.
   */
  protected getListItem(
    option: TOption | TGroup,
    isSelected?: boolean,
  ): string {
    let line = this.getListItemIndent(option);
    line += this.getListItemPointer(option, isSelected);
    line += this.getListItemIcon(option);
    line += this.getListItemLabel(option, isSelected);

    return line;
  }

  protected getListItemIndent(option: TOption | TGroup): string {
    const indentLevel = this.isSearching()
      ? option.indentLevel
      : this.hasParent() && !this.isBackButton(option)
      ? 1
      : 0;

    return this.settings.indent + " ".repeat(indentLevel);
  }

  protected getListItemPointer(
    option: TOption | TGroup,
    isSelected?: boolean,
  ): string {
    if (!isSelected) {
      return "  ";
    }

    if (this.isBackButton(option)) {
      return this.settings.backPointer + " ";
    } else if (isOptionGroup(option)) {
      return this.settings.groupPointer + " ";
    }

    return this.settings.listPointer + " ";
  }

  protected getListItemIcon(option: TOption | TGroup): string {
    if (this.isBackButton(option)) {
      return this.settings.groupOpenIcon
        ? this.settings.groupOpenIcon + " "
        : "";
    } else if (isOptionGroup(option)) {
      return this.settings.groupIcon ? this.settings.groupIcon + " " : "";
    }

    return "";
  }

  protected getListItemLabel(
    option: TOption | TGroup,
    isSelected?: boolean,
  ): string {
    let label = option.name;

    if (this.isBackButton(option)) {
      label = this.getBreadCrumb();
      label = isSelected && !option.disabled ? label : yellow(label);
    } else {
      label = isSelected && !option.disabled
        ? this.highlight(label, (val) => val)
        : this.highlight(label);
    }

    if (this.isBackButton(option) || isOptionGroup(option)) {
      label = bold(label);
    }

    return label;
  }

  protected getBreadCrumb(): string {
    if (!this.parentOptions.length || !this.settings.maxBreadcrumbItems) {
      return "";
    }
    const names = this.parentOptions.map((option) => option.name);
    const breadCrumb = names.length > this.settings.maxBreadcrumbItems
      ? [names[0], "..", ...names.slice(-this.settings.maxBreadcrumbItems + 1)]
      : names;

    return breadCrumb.join(this.settings.breadcrumbSeparator);
  }

  /** Get options row height. */
  protected getListHeight(): number {
    return Math.min(
      this.options.length,
      this.settings.maxRows || this.options.length,
    );
  }

  protected getListIndex(value?: TValue): number {
    return Math.max(
      0,
      typeof value === "undefined"
        ? this.options.findIndex((option: TOption | TGroup) =>
          !option.disabled
        ) || 0
        : this.options.findIndex((option: TOption | TGroup) =>
          isOption(option) && option.value === value
        ) ||
          0,
    );
  }

  protected getPageOffset(index: number): number {
    if (index === 0) {
      return 0;
    }
    const height: number = this.getListHeight();
    return Math.min(
      Math.floor(index / height) * height,
      this.options.length - height,
    );
  }

  /**
   * Find option by value.
   * @param value Value of the option.
   */
  protected getOptionByValue(
    value: TValue,
  ): TOption | undefined {
    const option = this.options.find((option) =>
      isOption(option) && option.value === value
    );

    return option && isOptionGroup(option) ? undefined : option;
  }

  /** Read user input. */
  protected read(): Promise<boolean> {
    if (!this.settings.search) {
      this.settings.tty.cursorHide();
    }
    return super.read();
  }

  protected selectSearch() {
    this.listIndex = -1;
  }

  protected isSearchSelected(): boolean {
    return this.listIndex === -1;
  }

  /**
   * Handle user input event.
   * @param event Key event.
   */
  protected async handleEvent(event: KeyCode): Promise<void> {
    if (
      this.isKey(this.settings.keys, "open", event) &&
      isOptionGroup(this.selectedOption) &&
      !this.isSearchSelected()
    ) {
      if (this.isBackButton(this.selectedOption)) {
        this.selectNext();
      } else {
        this.submitGroupOption(this.selectedOption);
      }
    } else if (
      this.isKey(this.settings.keys, "back", event) &&
      (this.isBackButton(this.selectedOption) || event.name === "escape") &&
      !this.isSearchSelected()
    ) {
      this.submitBackButton();
    } else if (this.isKey(this.settings.keys, "next", event)) {
      this.selectNext();
    } else if (this.isKey(this.settings.keys, "previous", event)) {
      this.selectPrevious();
    } else if (
      this.isKey(this.settings.keys, "nextPage", event) &&
      !this.isSearchSelected()
    ) {
      this.selectNextPage();
    } else if (
      this.isKey(this.settings.keys, "previousPage", event) &&
      !this.isSearchSelected()
    ) {
      this.selectPreviousPage();
    } else {
      await super.handleEvent(event);
    }
  }

  protected async submit(): Promise<void> {
    if (this.isSearchSelected()) {
      this.selectNext();
      return;
    }
    await super.submit();
  }

  protected moveCursorLeft(): void {
    if (this.settings.search) {
      super.moveCursorLeft();
    }
  }

  protected moveCursorRight(): void {
    if (this.settings.search) {
      super.moveCursorRight();
    }
  }

  protected deleteChar(): void {
    if (this.settings.search) {
      super.deleteChar();
    }
  }

  protected deleteCharRight(): void {
    if (this.settings.search) {
      super.deleteCharRight();
      this.match();
    }
  }

  protected addChar(char: string): void {
    if (this.settings.search) {
      super.addChar(char);
      this.match();
    }
  }

  /** Select previous option. */
  protected selectPrevious(loop = true): void {
    if (this.options.length < 2 && !this.isSearchSelected()) {
      return;
    }
    if (this.listIndex > 0) {
      this.listIndex--;
      if (this.listIndex < this.listOffset) {
        this.listOffset--;
      }
      if (this.selectedOption?.disabled) {
        this.selectPrevious();
      }
    } else if (
      this.settings.search && this.listIndex === 0 &&
      this.getCurrentInputValue().length
    ) {
      this.listIndex = -1;
    } else if (loop) {
      this.listIndex = this.options.length - 1;
      this.listOffset = this.options.length - this.getListHeight();
      if (this.selectedOption?.disabled) {
        this.selectPrevious();
      }
    }
  }

  /** Select next option. */
  protected selectNext(loop = true): void {
    if (this.options.length < 2 && !this.isSearchSelected()) {
      return;
    }
    if (this.listIndex < this.options.length - 1) {
      this.listIndex++;
      if (this.listIndex >= this.listOffset + this.getListHeight()) {
        this.listOffset++;
      }
      if (this.selectedOption?.disabled) {
        this.selectNext();
      }
    } else if (
      this.settings.search && this.listIndex === this.options.length - 1 &&
      this.getCurrentInputValue().length
    ) {
      this.listIndex = -1;
    } else if (loop) {
      this.listIndex = this.listOffset = 0;
      if (this.selectedOption?.disabled) {
        this.selectNext();
      }
    }
  }

  /** Select previous page. */
  protected selectPreviousPage(): void {
    if (this.options?.length) {
      const height: number = this.getListHeight();
      if (this.listOffset >= height) {
        this.listIndex -= height;
        this.listOffset -= height;
      } else if (this.listOffset > 0) {
        this.listIndex -= this.listOffset;
        this.listOffset = 0;
      } else {
        this.listIndex = 0;
      }
      if (this.selectedOption?.disabled) {
        this.selectPrevious(false);
      }
      if (this.selectedOption?.disabled) {
        this.selectNext(false);
      }
    }
  }

  /** Select next page. */
  protected selectNextPage(): void {
    if (this.options?.length) {
      const height: number = this.getListHeight();
      if (this.listOffset + height + height < this.options.length) {
        this.listIndex += height;
        this.listOffset += height;
      } else if (this.listOffset + height < this.options.length) {
        const offset = this.options.length - height;
        this.listIndex += offset - this.listOffset;
        this.listOffset = offset;
      } else {
        this.listIndex = this.options.length - 1;
      }
      if (this.selectedOption?.disabled) {
        this.selectNext(false);
      }
      if (this.selectedOption?.disabled) {
        this.selectPrevious(false);
      }
    }
  }
}

export function isOption<
  TValue,
  TOption extends GenericListOption<TValue>,
>(
  option:
    | TOption
    | GenericListOptionGroup<TValue, GenericListOption<TValue>>
    | GenericListSeparatorOption
    | undefined,
): option is TOption {
  return !!option && typeof option === "object" && "value" in option;
}

export function isOptionGroup<
  TValue,
  TGroup extends GenericListOptionGroup<TValue, GenericListOption<TValue>>,
>(
  option:
    | TGroup
    | TValue
    | GenericListOption<TValue>
    | GenericListSeparatorOption
    | undefined,
): option is TGroup {
  return option !== null && typeof option === "object" && "options" in option &&
    Array.isArray(option.options);
}

function matchOptions<
  TValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
>(
  searchInput: string,
  options: Array<TOption | TGroup>,
): Array<MatchedOption<TValue, TOption, TGroup>> {
  const matched: Array<MatchedOption<TValue, TOption, TGroup>> = [];

  for (const option of options) {
    if (isOptionGroup(option)) {
      const children = matchOptions<TValue, TOption, TGroup>(
        searchInput,
        option.options,
      )
        .sort(sortByDistance);

      if (children.length) {
        matched.push({
          option,
          distance: Math.min(...children.map((item) => item.distance)),
          children,
        });
        continue;
      }
    }

    if (matchOption(searchInput, option)) {
      matched.push({
        option,
        distance: levenshteinDistance(option.name, searchInput),
        children: [],
      });
    }
  }

  return matched.sort(sortByDistance);

  function sortByDistance(
    a: MatchedOption<TValue, TOption, TGroup>,
    b: MatchedOption<TValue, TOption, TGroup>,
  ): number {
    return a.distance - b.distance;
  }
}

function matchOption<
  TValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
>(
  inputString: string,
  option: TOption | TGroup,
): boolean {
  return matchInput(inputString, option.name) || (
    isOption(option) &&
    option.name !== option.value &&
    matchInput(inputString, String(option.value))
  );
}

function matchInput(inputString: string, value: string): boolean {
  return stripAnsiCode(value)
    .toLowerCase()
    .includes(inputString);
}

function flatMatchedOptions<
  TValue,
  TOption extends GenericListOptionSettings<TValue>,
  TGroup extends GenericListOptionGroupSettings<TValue, TOption>,
>(
  matches: Array<MatchedOption<TValue, TOption, TGroup>>,
  indentLevel = 0,
  result: Array<TOption | TGroup> = [],
): Array<TOption | TGroup> {
  for (const { option, children } of matches) {
    option.indentLevel = indentLevel;
    result.push(option);
    flatMatchedOptions(children, indentLevel + 1, result);
  }

  return result;
}

/**
 * GenericList options type.
 *
 * @deprecated Use `Array<string | GenericListOption>` instead.
 */
export type GenericListValueOptions = Array<string | GenericListOption<string>>;
/** @deprecated Use `Array<GenericListOptionSettings>` instead. */
export type GenericListValueSettings = Array<GenericListOptionSettings<string>>;
