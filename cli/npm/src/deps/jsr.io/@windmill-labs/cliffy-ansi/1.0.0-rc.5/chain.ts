/** Chainable ansi escape sequences. */
import type { ImageOptions } from "./ansi_escapes.js";

/** Chainable ansi escape method declarations. */
export interface Chain<TContext extends Chain<TContext>> {
  /** Add text. */
  text: (text: string) => TContext;
  toArray: () => Array<string>;
  /** Ring audio bell: `\u0007` */
  bel: TContext;
  /** Get cursor position. */
  cursorPosition: TContext;
  /**
   * Move cursor to x, y, counting from the top left corner.
   * @param x Position left.
   * @param y Position top.
   */
  cursorTo: (x: number, y?: number) => TContext;
  /**
   * Move cursor by offset.
   * @param x Offset left.
   * @param y Offset top.
   */
  cursorMove: (x: number, y: number) => TContext;
  /**
   * Move cursor up by n lines.
   * @param count Number of lines.
   */
  cursorUp: TContext & ((count: number) => TContext);
  /**
   * Move cursor down by n lines.
   * @param count Number of lines.
   */
  cursorDown: TContext & ((count: number) => TContext);
  /**
   * Move cursor forward by n lines.
   * @param count Number of lines.
   */
  cursorForward: TContext & ((count: number) => TContext);
  /**
   * Move cursor backward by n lines.
   * @param count Number of lines.
   */
  cursorBackward: TContext & ((count: number) => TContext);
  /**
   * Move cursor to the beginning of the line n lines down.
   * @param count Number of lines.
   */
  cursorNextLine: TContext & ((count: number) => TContext);
  /**
   * Move cursor to the beginning of the line n lines up.
   * @param count Number of lines.
   */
  cursorPrevLine: TContext & ((count: number) => TContext);
  /** Move cursor to first column of current row. */
  cursorLeft: TContext;
  /** Hide cursor. */
  cursorHide: TContext;
  /** Show cursor. */
  cursorShow: TContext;
  /** Save cursor. */
  cursorSave: TContext;
  /** Restore cursor. */
  cursorRestore: TContext;
  /**
   * Scroll window up by n lines.
   * @param count Number of lines.
   */
  scrollUp: TContext & ((count: number) => TContext);
  /**
   * Scroll window down by n lines.
   * @param count Number of lines.
   */
  scrollDown: TContext & ((count: number) => TContext);
  /** Clear screen. */
  eraseScreen: TContext;
  /**
   * Clear screen up by n lines.
   * @param count Number of lines.
   */
  eraseUp: TContext & ((count: number) => TContext);
  /**
   * Clear screen down by n lines.
   * @param count Number of lines.
   */
  eraseDown: TContext & ((count: number) => TContext);
  /** Clear current line. */
  eraseLine: TContext;
  /** Clear to line end. */
  eraseLineEnd: TContext;
  /** Clear to line start. */
  eraseLineStart: TContext;
  /**
   * Clear screen and move cursor by n lines up and move cursor to first column.
   * @param count Number of lines.
   */
  eraseLines: (count: number) => TContext;
  /** Clear the terminal screen. (Viewport) */
  clearScreen: TContext;
  /**
   * Clear the whole terminal, including scrollback buffer.
   * (Not just the visible part of it).
   */
  clearTerminal: TContext;
  /**
   * Create link.
   *
   * @param text Link text.
   * @param url Link url.
   *
   * ```ts
   * import { ansi } from "@cliffy/ansi";
   *
   * console.log(
   *   ansi.link("Click me.", "https://deno.land"),
   * );
   * ```
   */
  link: (text: string, url: string) => TContext;
  /**
   * Create image.
   *
   * @param buffer  Image buffer.
   * @param options Image options.
   *
   * ```ts
   * import { ansi } from "@cliffy/ansi";
   *
   * const response = await fetch("https://deno.land/images/hashrock_simple.png");
   * const imageBuffer: ArrayBuffer = await response.arrayBuffer();
   * console.log(
   *   ansi.image(imageBuffer),
   * );
   * ```
   */
  image: (buffer: string | ArrayBuffer, options?: ImageOptions) => TContext;
}
