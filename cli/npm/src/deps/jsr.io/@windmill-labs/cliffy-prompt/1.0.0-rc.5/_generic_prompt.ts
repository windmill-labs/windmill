import * as dntShim from "../../../../../_dnt.shims.js";
import type { Cursor } from "../../cliffy-ansi/1.0.0-rc.5/cursor_position.js";
import { type Tty, tty } from "../../cliffy-ansi/1.0.0-rc.5/tty.js";
import { type KeyCode, parse } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import {
  bold,
  brightBlue,
  dim,
  green,
  italic,
  red,
  stripAnsiCode,
  yellow,
} from "../../../@std/fmt/0.225.6/colors.js";
import type { Reader, WriterSync } from "../../../@std/io/0.224.7/types.js";
import { readSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/read_sync.js";
import { writeSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import { Figures } from "./_figures.js";
import { exit } from "../../cliffy-internal/1.0.0-rc.5/runtime/exit.js";
import { getColumns } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_columns.js";
import { isTerminal } from "../../cliffy-internal/1.0.0-rc.5/runtime/is_terminal.js";
import { read } from "../../cliffy-internal/1.0.0-rc.5/runtime/read.js";
import { setRaw } from "../../cliffy-internal/1.0.0-rc.5/runtime/set_raw.js";

/** Static generic prompt interface. */
export interface StaticGenericPrompt<TValue, TOptions> {
  /**
   * Inject prompt value. If called, the prompt doesn't prompt for an input and
   * returns immediately the injected value. Can be used for unit tests or pre
   * selections.
   *
   * @param value The injected value.
   */
  inject?(value: TValue): void;

  /**
   * Execute the prompt with the given options.
   *
   * @param options Prompt options.
   */
  prompt(options: TOptions): Promise<TValue>;
}

// deno-lint-ignore no-explicit-any
export type InferPromptOptions<TPrompt extends GenericPrompt<any, any>> =
  Parameters<
    TPrompt["getDefaultSettings"]
  >[0];

// deno-lint-ignore no-explicit-any
export type InferPromptValue<TPrompt extends GenericPrompt<any, any>> = Awaited<
  ReturnType<
    TPrompt["prompt"]
  >
>;

/** Generic prompt options. */
export interface GenericPromptOptions<TValue, TRawValue> {
  /** The prompt message. */
  message: string;
  /** The default value of the prompt. */
  default?: TValue;
  /** Don't show the default value after the prompt message. */
  hideDefault?: boolean;
  /** Validate prompt value. */
  validate?: (value: TRawValue) => ValidateResult;
  /** Transform input value to output value. */
  transform?: (value: TRawValue) => TValue | undefined;
  /** Show a hint below the prompt. */
  hint?: string;
  /** Change the prompt pointer. Default is `brightBlue("›")`. */
  pointer?: string;
  /** Indent the prompt. */
  indent?: string;
  /** Keymap to assign key names to prompt actions. */
  keys?: GenericPromptKeys;
  /** Enable cbreak mode. For more information see [Deno.SetRawOptions](https://deno.land/api@v1.42.1?s=Deno.SetRawOptions). */
  cbreak?: boolean;
  /** Change the prompt prefix. Default is: `yellow("? ")`. */
  prefix?: string;
  /** Change the prompt input stream. */
  reader?: Reader & {
    setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
    isTerminal(): boolean;
  };
  /** Change the prompt output stream. */
  writer?: WriterSync;
}

/** Generic prompt settings. */
export interface GenericPromptSettings<TValue, TRawValue>
  extends GenericPromptOptions<TValue, TRawValue> {
  pointer: string;
  indent: string;
  prefix: string;
  cbreak: boolean;
  tty: Tty;
  reader: Reader & {
    setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
    isTerminal(): boolean;
  };
  writer: WriterSync;
}

/** Prompt validation return tape. */
export type ValidateResult = string | boolean | Promise<string | boolean>;

/** Generic prompt keymap. */
export interface GenericPromptKeys {
  /** Submit keymap. Default is `["enter", "return"]`. */
  submit?: Array<string>;
}

/** Generic prompt representation. */
export abstract class GenericPrompt<
  TValue,
  TRawValue,
> {
  protected static injectedValue: unknown | undefined;
  protected abstract readonly settings: GenericPromptSettings<
    TValue,
    TRawValue
  >;
  protected readonly cursor: Cursor = {
    x: 0,
    y: 0,
  };
  #value: TValue | undefined;
  #lastError: string | undefined;
  #isFirstRun = true;
  #encoder = new TextEncoder();

  /**
   * Inject prompt value. If called, the prompt doesn't prompt for an input and
   * returns immediately the injected value. Can be used for unit tests or pre
   * selections.
   *
   * @param value Input value.
   */
  public static inject(value: unknown): void {
    GenericPrompt.injectedValue = value;
  }

  public getDefaultSettings(
    options: GenericPromptOptions<TValue, TRawValue>,
  ): GenericPromptSettings<TValue, TRawValue> {
    return {
      ...options,
      tty: tty({
        // Stdin is only used by getCursorPosition which we don't need.
        reader: { readSync, setRaw },
        writer: options.writer ?? { writeSync },
      }),
      cbreak: options.cbreak ?? false,
      reader: options.reader ?? { read, setRaw, isTerminal },
      writer: options.writer ?? { writeSync },
      pointer: options.pointer ?? brightBlue(Figures.POINTER_SMALL),
      prefix: options.prefix ?? yellow("? "),
      indent: options.indent ?? "",
      keys: {
        submit: ["enter", "return"],
        ...(options.keys ?? {}),
      },
    };
  }

  /** Execute the prompt. */
  public async prompt(): Promise<TValue> {
    try {
      return await this.#execute();
    } finally {
      this.settings.tty.cursorShow();
    }
  }

  /** Clear prompt output. */
  protected clear(): void {
    this.settings.tty.cursorLeft.eraseDown();
  }

  /** Execute the prompt. */
  #execute = async (): Promise<TValue> => {
    // Throw errors on unit tests.
    if (typeof GenericPrompt.injectedValue !== "undefined" && this.#lastError) {
      throw new Error(this.error());
    }

    await this.render();
    this.#lastError = undefined;

    if (!await this.read()) {
      return this.#execute();
    }

    if (typeof this.#value === "undefined") {
      throw new Error("internal error: failed to read value");
    }

    this.clear();
    const successMessage: string | undefined = this.success(this.#value);

    if (successMessage) {
      this.settings.writer.writeSync(
        this.#encoder.encode(successMessage + "\n"),
      );
    }

    GenericPrompt.injectedValue = undefined;
    this.settings.tty.cursorShow();

    return this.#value;
  };

  /** Render prompt. */
  protected async render(): Promise<void> {
    const result: [string, string | undefined, string | undefined] =
      await Promise.all([
        this.message(),
        this.body?.(),
        this.footer(),
      ]);

    const content: string = result.filter(Boolean).join("\n");
    const lines = content.split("\n");

    const columns = getColumns();
    const linesCount: number = columns
      ? lines.reduce((prev, next) => {
        const length = stripAnsiCode(next).length;
        return prev + (length > columns ? Math.ceil(length / columns) : 1);
      }, 0)
      : content.split("\n").length;

    const y: number = linesCount - this.cursor.y - 1;

    if (!this.#isFirstRun || this.#lastError) {
      this.clear();
    }
    this.#isFirstRun = false;
    this.settings.writer.writeSync(this.#encoder.encode(content));

    if (y) {
      this.settings.tty.cursorUp(y);
    }
    this.settings.tty.cursorTo(this.cursor.x);
  }

  /** Read user input from stdin, handle events and validate user input. */
  protected async read(): Promise<boolean> {
    if (typeof GenericPrompt.injectedValue !== "undefined") {
      const value: TRawValue = GenericPrompt.injectedValue as TRawValue;
      await this.#validateValue(value);
    } else {
      const events: Array<KeyCode> = await this.#readKey();

      if (!events.length) {
        return false;
      }

      for (const event of events) {
        await this.handleEvent(event);
      }
    }

    return typeof this.#value !== "undefined";
  }

  protected submit(): Promise<void> {
    return this.#validateValue(this.getValue());
  }

  protected message(): string {
    return `${this.settings.indent}${this.settings.prefix}` +
      bold(this.settings.message) + this.defaults();
  }

  protected defaults(): string {
    let defaultMessage = "";
    if (
      typeof this.settings.default !== "undefined" && !this.settings.hideDefault
    ) {
      defaultMessage += dim(` (${this.format(this.settings.default)})`);
    }
    return defaultMessage;
  }

  /** Get prompt success message. */
  protected success(value: TValue): string | undefined {
    return `${this.settings.indent}${this.settings.prefix}` +
      bold(this.settings.message) + this.defaults() +
      " " + this.settings.pointer +
      " " + green(this.format(value));
  }

  protected body?(): string | undefined | Promise<string | undefined>;

  protected footer(): string | undefined {
    return this.error() ?? this.hint();
  }

  protected error(): string | undefined {
    return this.#lastError
      ? this.settings.indent + red(bold(`${Figures.CROSS} `) + this.#lastError)
      : undefined;
  }

  protected hint(): string | undefined {
    return this.settings.hint
      ? this.settings.indent +
        italic(brightBlue(dim(`${Figures.POINTER} `) + this.settings.hint))
      : undefined;
  }

  protected setErrorMessage(message: string) {
    this.#lastError = message;
  }

  /**
   * Handle user input event.
   * @param event Key event.
   */
  protected async handleEvent(event: KeyCode): Promise<void> {
    switch (true) {
      case event.name === "c" && event.ctrl:
        this.clear();
        this.settings.tty.cursorShow();
        exit(130);
        return;
      case this.isKey(this.settings.keys, "submit", event):
        await this.submit();
        break;
    }
  }

  /**
   * Map input value to output value.
   * @param value Input value.
   * @return Output value.
   */
  protected abstract transform(value: TRawValue): TValue | undefined;

  /**
   * Validate input value.
   * @param value User input value.
   * @return True on success, false or error message on error.
   */
  protected abstract validate(value: TRawValue): ValidateResult;

  /**
   * Format output value.
   * @param value Output value.
   */
  protected abstract format(value: TValue): string;

  /** Get input value. */
  protected abstract getValue(): TRawValue;

  /** Read user input from stdin and pars ansi codes. */
  #readKey = async (): Promise<Array<KeyCode>> => {
    const data: Uint8Array = await this.#readChar();

    return data.length ? parse(data) : [];
  };

  /** Read user input from stdin. */
  #readChar = async (): Promise<Uint8Array> => {
    const buffer = new Uint8Array(8);
    const isTty = this.settings.reader.isTerminal();

    if (isTty) {
      this.settings.reader.setRaw(
        true,
        { cbreak: this.settings.cbreak },
      );
    }
    const nread: number | null = await this.settings.reader.read(buffer);

    if (isTty) {
      this.settings.reader.setRaw(false);
    }

    if (nread === null) {
      return buffer;
    }

    return buffer.subarray(0, nread);
  };

  /**
   * Map input value to output value. If a custom transform handler ist set, the
   * custom handler will be executed, otherwise the default transform handler
   * from the prompt will be executed.
   * @param value The value to transform.
   */
  #transformValue = (value: TRawValue): TValue | undefined => {
    return this.settings.transform
      ? this.settings.transform(value)
      : this.transform(value);
  };

  /**
   * Validate input value. Set error message if validation fails and transform
   * output value on success.
   * If a default value is set, the default will be used as value without any
   * validation.
   * If a custom validation handler ist set, the custom handler will
   * be executed, otherwise a prompt specific default validation handler will be
   * executed.
   * @param value The value to validate.
   */
  #validateValue = async (value: TRawValue): Promise<void> => {
    if (!value && typeof this.settings.default !== "undefined") {
      this.#value = this.settings.default;
      return;
    }

    this.#value = undefined;
    this.#lastError = undefined;

    const validation =
      await (this.settings.validate
        ? this.settings.validate(value)
        : this.validate(value));

    if (validation === false) {
      this.#lastError = `Invalid answer.`;
    } else if (typeof validation === "string") {
      this.#lastError = validation;
    } else {
      this.#value = this.#transformValue(value);
    }
  };

  /**
   * Check if key event has given name or sequence.
   * @param keys  Key map.
   * @param name  Key name.
   * @param event Key event.
   */
  protected isKey<TKey extends unknown, TName extends keyof TKey>(
    keys: TKey | undefined,
    name: TName,
    event: KeyCode,
  ): boolean {
    // deno-lint-ignore no-explicit-any
    const keyNames: Array<unknown> | undefined = keys?.[name] as any;
    return typeof keyNames !== "undefined" && (
      (typeof event.name !== "undefined" &&
        keyNames.indexOf(event.name) !== -1) ||
      (typeof event.sequence !== "undefined" &&
        keyNames.indexOf(event.sequence) !== -1)
    );
  }
}
