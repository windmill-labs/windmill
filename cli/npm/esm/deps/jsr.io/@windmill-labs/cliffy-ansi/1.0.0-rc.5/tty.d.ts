import * as dntShim from "../../../../../_dnt.shims.js";
import type { Chain } from "./chain.js";
import { type Cursor } from "./cursor_position.js";
import type { ReaderSync, WriterSync } from "../../../@std/io/0.224.7/types.js";
/** Create new `Ansi` instance. */
export interface TtyOptions {
    writer?: WriterSync;
    reader?: ReaderSync & {
        setRaw(mode: boolean, options?: dntShim.Deno.SetRawOptions): void;
    };
}
/** Ansi instance returned by all ansi escape properties. */
export interface TtyChain extends Exclude<Chain<TtyChain>, "cursorPosition"> {
    /** Write ansi escape sequence. */
    (): void;
    /** Get current cursor position. */
    getCursorPosition(): Cursor;
}
/** Create new `Tty` instance. */
export type TtyFactory = (options?: TtyOptions) => Tty;
/**
 * Chainable ansi escape sequences.
 * If invoked as method, a new Tty instance will be returned.
 */
export type Tty = TtyFactory & TtyChain;
/**
 * Chainable ansi escape sequences.
 * If invoked as method, a new Tty instance will be returned.
 *
 * ```ts
 * import { tty } from "@cliffy/ansi/tty";
 *
 * tty.cursorTo(0, 0).eraseScreen();
 * ```
 */
export declare const tty: Tty;
//# sourceMappingURL=tty.d.ts.map