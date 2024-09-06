import { isBrowserWindow, isDeno, isNodeLike } from "../env.ts";

export type ProgressState = {
    /**
     * Once set, the progress bar will be updated to display the given
     * percentage. Valid values are between `0` and `100`.
     */
    percent?: number;
    /**
     * Once set, the progress dialog will be updated to display the given message.
     */
    message?: string;
};

export type ProgressFunc<T> = (set: (state: ProgressState) => void, signal: AbortSignal) => Promise<T>;
export type ProgressAbortHandler<T> = () => T | never | Promise<T | never>;

/**
 * Displays a dialog with a progress bar indicating the ongoing state of the
 * `fn` function, and to wait until the job finishes or the user cancels the
 * dialog.
 * 
 * @param onAbort If provided, the dialog will show a cancel button (or listen
 * for Escape in CLI) that allows the user to abort the task. This function can
 * either return a default/fallback result or throw an error to indicate the
 * cancellation.
 * 
 * @example
 * ```ts
 * // default usage
 * import { progress } from "@ayonli/jsext/dialog";
 * 
 * const result = await progress("Processing...", async () => {
 *     // ... some long-running task
 *     return { ok: true };
 * });
 * 
 * console.log(result); // { ok: true }
 * ```
 * 
 * @example
 * ```ts
 * // update state
 * import { progress } from "@ayonli/jsext/dialog";
 * 
 * const result = await progress("Processing...", async (set) => {
 *     set({ percent: 0 });
 *     // ... some long-running task
 *     set({ percent: 50, message: "Halfway there!" });
 *     // ... some long-running task
 *     set({ percent: 100 });
 * 
 *     return { ok: true };
 * });
 * 
 * console.log(result); // { ok: true }
 * ```
 * 
 * @example
 * ```ts
 * // abortable
 * import { progress } from "@ayonli/jsext/dialog";
 * 
 * const result = await progress("Processing...", async (set, signal) => {
 *     set({ percent: 0 });
 * 
 *     if (!signal.aborted) {
 *         // ... some long-running task
 *         set({ percent: 50, message: "Halfway there!" });
 *     }
 * 
 *     if (!signal.aborted) {
 *         // ... some long-running task
 *         set({ percent: 100 });
 *     }
 * 
 *     return { ok: true };
 * }, () => {
 *     return { ok: false };
 * });
 * 
 * console.log(result); // { ok: true } or { ok: false }
 * ```
 */
export default async function progress<T>(
    message: string,
    fn: ProgressFunc<T>,
    onAbort: ProgressAbortHandler<T> | undefined = undefined
): Promise<T | null> {
    const ctrl = new AbortController();
    const signal = ctrl.signal;
    let fallback: { value: T; } | null = null;
    const abort = !onAbort ? undefined : async () => {
        try {
            const result = await onAbort();
            fallback = { value: result };
            ctrl.abort();
        } catch (err) {
            ctrl.abort(err);
        }
    };
    const listenForAbort = !onAbort ? undefined : () => new Promise<T>((resolve, reject) => {
        signal.addEventListener("abort", () => {
            if (fallback) {
                resolve(fallback.value);
            } else {
                reject(signal.reason);
            }
        });
    });

    if (isBrowserWindow) {
        const { progressInBrowser } = await import("./browser/index.ts");
        return await progressInBrowser(message, fn, { signal, abort, listenForAbort });
    } else if (isDeno || isNodeLike) {
        const { lockStdin } = await import("../cli.ts");
        const { handleTerminalProgress } = await import("./terminal/progress.ts");
        return await lockStdin(() => handleTerminalProgress(message, fn, {
            signal,
            abort,
            listenForAbort,
        }));
    } else {
        throw new Error("Unsupported runtime");
    }
}
