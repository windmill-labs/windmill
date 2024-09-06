var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var __classPrivateFieldSet = (this && this.__classPrivateFieldSet) || function (receiver, state, value, kind, f) {
    if (kind === "m") throw new TypeError("Private method is not writable");
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a setter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot write private member to an object whose class did not declare it");
    return (kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value)), value;
};
var _a, _GenericPrompt_value, _GenericPrompt_lastError, _GenericPrompt_isFirstRun, _GenericPrompt_encoder, _GenericPrompt_execute, _GenericPrompt_readKey, _GenericPrompt_readChar, _GenericPrompt_transformValue, _GenericPrompt_validateValue;
import { tty } from "../../cliffy-ansi/1.0.0-rc.5/tty.js";
import { parse } from "../../cliffy-keycode/1.0.0-rc.5/mod.js";
import { bold, brightBlue, dim, green, italic, red, stripAnsiCode, yellow, } from "../../../@std/fmt/0.225.6/colors.js";
import { readSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/read_sync.js";
import { writeSync } from "../../cliffy-internal/1.0.0-rc.5/runtime/write_sync.js";
import { Figures } from "./_figures.js";
import { exit } from "../../cliffy-internal/1.0.0-rc.5/runtime/exit.js";
import { getColumns } from "../../cliffy-internal/1.0.0-rc.5/runtime/get_columns.js";
import { isTerminal } from "../../cliffy-internal/1.0.0-rc.5/runtime/is_terminal.js";
import { read } from "../../cliffy-internal/1.0.0-rc.5/runtime/read.js";
import { setRaw } from "../../cliffy-internal/1.0.0-rc.5/runtime/set_raw.js";
/** Generic prompt representation. */
export class GenericPrompt {
    constructor() {
        Object.defineProperty(this, "cursor", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {
                x: 0,
                y: 0,
            }
        });
        _GenericPrompt_value.set(this, void 0);
        _GenericPrompt_lastError.set(this, void 0);
        _GenericPrompt_isFirstRun.set(this, true);
        _GenericPrompt_encoder.set(this, new TextEncoder());
        /** Execute the prompt. */
        _GenericPrompt_execute.set(this, async () => {
            // Throw errors on unit tests.
            if (typeof _a.injectedValue !== "undefined" && __classPrivateFieldGet(this, _GenericPrompt_lastError, "f")) {
                throw new Error(this.error());
            }
            await this.render();
            __classPrivateFieldSet(this, _GenericPrompt_lastError, undefined, "f");
            if (!await this.read()) {
                return __classPrivateFieldGet(this, _GenericPrompt_execute, "f").call(this);
            }
            if (typeof __classPrivateFieldGet(this, _GenericPrompt_value, "f") === "undefined") {
                throw new Error("internal error: failed to read value");
            }
            this.clear();
            const successMessage = this.success(__classPrivateFieldGet(this, _GenericPrompt_value, "f"));
            if (successMessage) {
                this.settings.writer.writeSync(__classPrivateFieldGet(this, _GenericPrompt_encoder, "f").encode(successMessage + "\n"));
            }
            _a.injectedValue = undefined;
            this.settings.tty.cursorShow();
            return __classPrivateFieldGet(this, _GenericPrompt_value, "f");
        });
        /** Read user input from stdin and pars ansi codes. */
        _GenericPrompt_readKey.set(this, async () => {
            const data = await __classPrivateFieldGet(this, _GenericPrompt_readChar, "f").call(this);
            return data.length ? parse(data) : [];
        });
        /** Read user input from stdin. */
        _GenericPrompt_readChar.set(this, async () => {
            const buffer = new Uint8Array(8);
            const isTty = this.settings.reader.isTerminal();
            if (isTty) {
                this.settings.reader.setRaw(true, { cbreak: this.settings.cbreak });
            }
            const nread = await this.settings.reader.read(buffer);
            if (isTty) {
                this.settings.reader.setRaw(false);
            }
            if (nread === null) {
                return buffer;
            }
            return buffer.subarray(0, nread);
        });
        /**
         * Map input value to output value. If a custom transform handler ist set, the
         * custom handler will be executed, otherwise the default transform handler
         * from the prompt will be executed.
         * @param value The value to transform.
         */
        _GenericPrompt_transformValue.set(this, (value) => {
            return this.settings.transform
                ? this.settings.transform(value)
                : this.transform(value);
        });
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
        _GenericPrompt_validateValue.set(this, async (value) => {
            if (!value && typeof this.settings.default !== "undefined") {
                __classPrivateFieldSet(this, _GenericPrompt_value, this.settings.default, "f");
                return;
            }
            __classPrivateFieldSet(this, _GenericPrompt_value, undefined, "f");
            __classPrivateFieldSet(this, _GenericPrompt_lastError, undefined, "f");
            const validation = await (this.settings.validate
                ? this.settings.validate(value)
                : this.validate(value));
            if (validation === false) {
                __classPrivateFieldSet(this, _GenericPrompt_lastError, `Invalid answer.`, "f");
            }
            else if (typeof validation === "string") {
                __classPrivateFieldSet(this, _GenericPrompt_lastError, validation, "f");
            }
            else {
                __classPrivateFieldSet(this, _GenericPrompt_value, __classPrivateFieldGet(this, _GenericPrompt_transformValue, "f").call(this, value), "f");
            }
        });
    }
    /**
     * Inject prompt value. If called, the prompt doesn't prompt for an input and
     * returns immediately the injected value. Can be used for unit tests or pre
     * selections.
     *
     * @param value Input value.
     */
    static inject(value) {
        _a.injectedValue = value;
    }
    getDefaultSettings(options) {
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
    async prompt() {
        try {
            return await __classPrivateFieldGet(this, _GenericPrompt_execute, "f").call(this);
        }
        finally {
            this.settings.tty.cursorShow();
        }
    }
    /** Clear prompt output. */
    clear() {
        this.settings.tty.cursorLeft.eraseDown();
    }
    /** Render prompt. */
    async render() {
        const result = await Promise.all([
            this.message(),
            this.body?.(),
            this.footer(),
        ]);
        const content = result.filter(Boolean).join("\n");
        const lines = content.split("\n");
        const columns = getColumns();
        const linesCount = columns
            ? lines.reduce((prev, next) => {
                const length = stripAnsiCode(next).length;
                return prev + (length > columns ? Math.ceil(length / columns) : 1);
            }, 0)
            : content.split("\n").length;
        const y = linesCount - this.cursor.y - 1;
        if (!__classPrivateFieldGet(this, _GenericPrompt_isFirstRun, "f") || __classPrivateFieldGet(this, _GenericPrompt_lastError, "f")) {
            this.clear();
        }
        __classPrivateFieldSet(this, _GenericPrompt_isFirstRun, false, "f");
        this.settings.writer.writeSync(__classPrivateFieldGet(this, _GenericPrompt_encoder, "f").encode(content));
        if (y) {
            this.settings.tty.cursorUp(y);
        }
        this.settings.tty.cursorTo(this.cursor.x);
    }
    /** Read user input from stdin, handle events and validate user input. */
    async read() {
        if (typeof _a.injectedValue !== "undefined") {
            const value = _a.injectedValue;
            await __classPrivateFieldGet(this, _GenericPrompt_validateValue, "f").call(this, value);
        }
        else {
            const events = await __classPrivateFieldGet(this, _GenericPrompt_readKey, "f").call(this);
            if (!events.length) {
                return false;
            }
            for (const event of events) {
                await this.handleEvent(event);
            }
        }
        return typeof __classPrivateFieldGet(this, _GenericPrompt_value, "f") !== "undefined";
    }
    submit() {
        return __classPrivateFieldGet(this, _GenericPrompt_validateValue, "f").call(this, this.getValue());
    }
    message() {
        return `${this.settings.indent}${this.settings.prefix}` +
            bold(this.settings.message) + this.defaults();
    }
    defaults() {
        let defaultMessage = "";
        if (typeof this.settings.default !== "undefined" && !this.settings.hideDefault) {
            defaultMessage += dim(` (${this.format(this.settings.default)})`);
        }
        return defaultMessage;
    }
    /** Get prompt success message. */
    success(value) {
        return `${this.settings.indent}${this.settings.prefix}` +
            bold(this.settings.message) + this.defaults() +
            " " + this.settings.pointer +
            " " + green(this.format(value));
    }
    footer() {
        return this.error() ?? this.hint();
    }
    error() {
        return __classPrivateFieldGet(this, _GenericPrompt_lastError, "f")
            ? this.settings.indent + red(bold(`${Figures.CROSS} `) + __classPrivateFieldGet(this, _GenericPrompt_lastError, "f"))
            : undefined;
    }
    hint() {
        return this.settings.hint
            ? this.settings.indent +
                italic(brightBlue(dim(`${Figures.POINTER} `) + this.settings.hint))
            : undefined;
    }
    setErrorMessage(message) {
        __classPrivateFieldSet(this, _GenericPrompt_lastError, message, "f");
    }
    /**
     * Handle user input event.
     * @param event Key event.
     */
    async handleEvent(event) {
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
     * Check if key event has given name or sequence.
     * @param keys  Key map.
     * @param name  Key name.
     * @param event Key event.
     */
    isKey(keys, name, event) {
        // deno-lint-ignore no-explicit-any
        const keyNames = keys?.[name];
        return typeof keyNames !== "undefined" && ((typeof event.name !== "undefined" &&
            keyNames.indexOf(event.name) !== -1) ||
            (typeof event.sequence !== "undefined" &&
                keyNames.indexOf(event.sequence) !== -1));
    }
}
_a = GenericPrompt, _GenericPrompt_value = new WeakMap(), _GenericPrompt_lastError = new WeakMap(), _GenericPrompt_isFirstRun = new WeakMap(), _GenericPrompt_encoder = new WeakMap(), _GenericPrompt_execute = new WeakMap(), _GenericPrompt_readKey = new WeakMap(), _GenericPrompt_readChar = new WeakMap(), _GenericPrompt_transformValue = new WeakMap(), _GenericPrompt_validateValue = new WeakMap();
