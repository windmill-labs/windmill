import type { Direction } from "./cell.js";

/** Column options. */
export interface ColumnOptions {
  /** Enable/disable cell border. */
  border?: boolean;
  /** Cell cell alignment direction. */
  align?: Direction;
  /** Set min column width. */
  minWidth?: number;
  /** Set max column width. */
  maxWidth?: number;
  /** Set cell padding. */
  padding?: number;
}

/**
 * Column representation.
 *
 * Can be used to customize a single column.
 *
 * ```ts
 * import { Column, Table } from "./mod.ts";
 *
 * new Table()
 *   .body([
 *     ["Foo", "bar"],
 *     ["Beep", "Boop"],
 *   ])
 *   .column(0, new Column().border())
 *   .render();
 * ```
 */
export class Column {
  /**
   * Create a new cell from column options or an existing column.
   * @param options
   */
  static from(options: ColumnOptions | Column): Column {
    const opts = options instanceof Column ? options.opts : options;
    return new Column().options(opts);
  }

  protected opts: ColumnOptions = {};

  /** Set column options. */
  options(options: ColumnOptions): this {
    Object.assign(this.opts, options);
    return this;
  }

  /** Set min column width. */
  minWidth(width: number): this {
    this.opts.minWidth = width;
    return this;
  }

  /** Set max column width. */
  maxWidth(width: number): this {
    this.opts.maxWidth = width;
    return this;
  }

  /** Set column border. */
  border(border = true): this {
    this.opts.border = border;
    return this;
  }

  /** Set column padding. */
  padding(padding: number): this {
    this.opts.padding = padding;
    return this;
  }

  /** Set column alignment. */
  align(direction: Direction): this {
    this.opts.align = direction;
    return this;
  }

  /** Get min column width. */
  getMinWidth(): number | undefined {
    return this.opts.minWidth;
  }

  /** Get max column width. */
  getMaxWidth(): number | undefined {
    return this.opts.maxWidth;
  }

  /** Get column border. */
  getBorder(): boolean | undefined {
    return this.opts.border;
  }

  /** Get column padding. */
  getPadding(): number | undefined {
    return this.opts.padding;
  }

  /** Get column alignment. */
  getAlign(): Direction | undefined {
    return this.opts.align;
  }
}
