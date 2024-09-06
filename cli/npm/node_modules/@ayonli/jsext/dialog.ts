/**
 * Asynchronous dialog functions for both browsers and terminals.
 * 
 * This includes `alert`, `confirm`, `prompt` and other non-standard dialogs.
 * @experimental
 * @module
 */

import progress from "./dialog/progress.ts";
import type { ProgressState, ProgressFunc, ProgressAbortHandler } from "./dialog/progress.ts";
import {
    type FileDialogOptions,
    type PickFileOptions,
    type SaveFileOptions,
    type DownloadFileOptions,
    openFile,
    openFiles,
    openDirectory,
    pickFile,
    pickFiles,
    pickDirectory,
    saveFile,
    downloadFile,
} from "./dialog/file.ts";
import { isBrowserWindow, isDeno, isNodeLike } from "./env.ts";

export type { FileDialogOptions, PickFileOptions, SaveFileOptions, DownloadFileOptions };
export {
    openFile,
    openFiles,
    openDirectory,
    pickFile,
    pickFiles,
    pickDirectory,
    saveFile,
    downloadFile,
};
export { progress, ProgressState, ProgressFunc, ProgressAbortHandler };

/**
 * Options for dialog functions such as {@link alert}, {@link confirm} and
 * {@link prompt}.
 */
export interface DialogOptions {
    /**
     * By default, a GUI dialog is displayed in the browser, and text mode is
     * used in the terminal. Set this option to `true` will force the program
     * to always display a GUI dialog, even in the terminal.
     * 
     * When in the terminal, the GUI dialog is rendered with the OS's native
     * dialog. If the dialog is failed to display, an error will be thrown.
     * 
     * This option is only functional in `Windows`, `macOS` and `Linux`, it is
     * ignored in other platforms and the browser.
     */
    gui?: boolean;
}

/**
 * Displays a dialog with a message, and to wait until the user dismisses the
 * dialog.
 * 
 * @example
 * ```ts
 * import { alert } from "@ayonli/jsext/dialog";
 * 
 * await alert("Hello, world!");
 * ```
 */
export async function alert(message: string, options: DialogOptions = {}): Promise<void> {
    if (isBrowserWindow) {
        const { alertInBrowser } = await import("./dialog/browser/index.ts");
        await alertInBrowser(message);
    } else if (isDeno || isNodeLike) {
        const { default: alertInTerminal } = await import("./dialog/terminal/alert.ts");
        await alertInTerminal(message, options);
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Displays a dialog with a message, and to wait until the user either confirms
 * or cancels the dialog.
 * 
 * @example
 * ```ts
 * import { confirm } from "@ayonli/jsext/dialog";
 * 
 * if (await confirm("Are you sure?")) {
 *     console.log("Confirmed");
 * } else {
 *     console.log("Canceled");
 * }
 * ```
 */
export async function confirm(message: string, options: DialogOptions = {}): Promise<boolean> {
    if (isBrowserWindow) {
        const { confirmInBrowser } = await import("./dialog/browser/index.ts");
        return await confirmInBrowser(message);
    } else if (isDeno || isNodeLike) {
        const { default: confirmInTerminal } = await import("./dialog/terminal/confirm.ts");
        return await confirmInTerminal(message, options);
    } else {
        throw new Error("Unsupported runtime");
    }
}

/**
 * Options for the {@link prompt} function.
 */
export interface PromptOptions extends DialogOptions {
    /**
     * The default value of the input box.
     */
    defaultValue?: string | undefined;
    /**
     * The type of the input box. The default value is `text`, when `password`
     * is specified, the input will be masked.
     */
    type?: "text" | "password";
    /**
     * Terminal only, used when `type` is `password`. The default value is
     * `*`, use an empty string if you don't want to show any character.
     * 
     * This option is ignored when `gui` is `true`.
     */
    mask?: string;
}

/**
 * Displays a dialog with a message prompting the user to input some text, and to
 * wait until the user either submits the text or cancels the dialog.
 * 
 * @example
 * ```ts
 * import { prompt } from "@ayonli/jsext/dialog";
 * 
 * const name = await prompt("What's your name?");
 * 
 * if (name) {
 *     console.log(`Hello, ${name}!`);
 * }
 * ```
 * 
 * @example
 * ```ts
 * // with default value
 * import { prompt } from "@ayonli/jsext/dialog";
 * 
 * const name = await prompt("What's your name?", "John Doe");
 * 
 * if (name) {
 *     console.log(`Hello, ${name}!`);
 * }
 * ```
 */
export async function prompt(
    message: string,
    defaultValue?: string | undefined
): Promise<string | null>;
/**
 * @example
 * ```ts
 * // input password
 * import { prompt } from "@ayonli/jsext/dialog";
 * 
 * const password = await prompt("Enter your password:", { type: "password" });
 * 
 * if (password) {
 *     console.log("Your password is:", password);
 * }
 * ```
 */
export async function prompt(message: string, options?: PromptOptions): Promise<string | null>;
export async function prompt(
    message: string,
    options: string | PromptOptions = ""
): Promise<string | null> {
    const defaultValue = typeof options === "string"
        ? options
        : options.defaultValue;
    const type = typeof options === "object"
        ? options.type ?? "text"
        : "text";
    const mask = type === "password"
        ? typeof options === "object" ? (options.mask ?? "*") : "*"
        : undefined;
    const gui = typeof options === "object" ? (options.gui ?? false) : false;

    if (isBrowserWindow) {
        const { promptInBrowser } = await import("./dialog/browser/index.ts");
        return await promptInBrowser(message, { type, defaultValue });
    } else if (isDeno || isNodeLike) {
        const { default: promptInTerminal } = await import("./dialog/terminal/prompt.ts");
        return await promptInTerminal(message, { defaultValue, type, mask, gui });
    } else {
        throw new Error("Unsupported runtime");
    }
}
