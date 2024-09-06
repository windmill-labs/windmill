import { GenericPrompt } from "./_generic_prompt.js";
import {
  GenericSuggestions,
  type GenericSuggestionsKeys,
  type GenericSuggestionsOptions,
  type GenericSuggestionsSettings,
} from "./_generic_suggestions.js";
import { dim } from "../../../@std/fmt/0.225.6/colors.js";

type UnsupportedOptions =
  | "files"
  | "complete"
  | "suggestions"
  | "list"
  | "info";

/** Confirm prompt options. */
export interface ConfirmOptions
  extends Omit<GenericSuggestionsOptions<boolean, string>, UnsupportedOptions> {
  /** Keymap to assign key names to prompt actions. */
  keys?: ConfirmKeys;
  /** Change active label. Default is `Yes`. */
  active?: string;
  /** Change inactive label. Default is `No`. */
  inactive?: string;
}

/** Confirm prompt settings. */
interface ConfirmSettings extends GenericSuggestionsSettings<boolean, string> {
  active: string;
  inactive: string;
  keys?: ConfirmKeys;
}

/** Confirm prompt keymap. */
export type ConfirmKeys = GenericSuggestionsKeys;

/**
 * Confirm prompt representation.
 *
 * ```ts
 * import { Confirm } from "./mod.ts";
 *
 * const confirmed: boolean = await Confirm.prompt("Please confirm");
 * ```
 */
export class Confirm extends GenericSuggestions<boolean, string> {
  protected readonly settings: ConfirmSettings;

  /** Execute the prompt with provided options. */
  public static prompt(
    options: string | ConfirmOptions,
  ): Promise<boolean> {
    return new this(options).prompt();
  }

  /**
   * Inject prompt value. If called, the prompt doesn't prompt for an input and
   * returns immediately the injected value. Can be used for unit tests or pre
   * selections.
   *
   * @param value Input value.
   */
  public static inject(value: string): void {
    GenericPrompt.inject(value);
  }

  constructor(options: string | ConfirmOptions) {
    super();
    if (typeof options === "string") {
      options = { message: options };
    }
    this.settings = this.getDefaultSettings(options);
  }

  public getDefaultSettings(options: ConfirmOptions): ConfirmSettings {
    return {
      ...super.getDefaultSettings(options),
      active: options.active || "Yes",
      inactive: options.inactive || "No",
      files: false,
      complete: undefined,
      suggestions: [
        options.active || "Yes",
        options.inactive || "No",
      ],
      list: false,
      info: false,
    };
  }

  protected defaults(): string {
    let defaultMessage = "";

    if (this.settings.default === true) {
      defaultMessage += this.settings.active[0].toUpperCase() + "/" +
        this.settings.inactive[0].toLowerCase();
    } else if (this.settings.default === false) {
      defaultMessage += this.settings.active[0].toLowerCase() + "/" +
        this.settings.inactive[0].toUpperCase();
    } else {
      defaultMessage += this.settings.active[0].toLowerCase() + "/" +
        this.settings.inactive[0].toLowerCase();
    }

    return defaultMessage ? dim(` (${defaultMessage})`) : "";
  }

  protected success(value: boolean): string | undefined {
    this.saveSuggestions(this.format(value));
    return super.success(value);
  }

  /** Get input input. */
  protected getValue(): string {
    return this.inputValue;
  }

  /**
   * Validate input value.
   * @param value User input value.
   * @return True on success, false or error message on error.
   */
  protected validate(value: string): boolean | string {
    return typeof value === "string" &&
      [
          this.settings.active[0].toLowerCase(),
          this.settings.active.toLowerCase(),
          this.settings.inactive[0].toLowerCase(),
          this.settings.inactive.toLowerCase(),
        ].indexOf(value.toLowerCase()) !== -1;
  }

  /**
   * Map input value to output value.
   * @param value Input value.
   * @return Output value.
   */
  protected transform(value: string): boolean | undefined {
    switch (value.toLowerCase()) {
      case this.settings.active[0].toLowerCase():
      case this.settings.active.toLowerCase():
        return true;
      case this.settings.inactive[0].toLowerCase():
      case this.settings.inactive.toLowerCase():
        return false;
    }
    return;
  }

  /**
   * Format output value.
   * @param value Output value.
   */
  protected format(value: boolean): string {
    return value ? this.settings.active : this.settings.inactive;
  }
}
