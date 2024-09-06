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
export declare class Column {
    /**
     * Create a new cell from column options or an existing column.
     * @param options
     */
    static from(options: ColumnOptions | Column): Column;
    protected opts: ColumnOptions;
    /** Set column options. */
    options(options: ColumnOptions): this;
    /** Set min column width. */
    minWidth(width: number): this;
    /** Set max column width. */
    maxWidth(width: number): this;
    /** Set column border. */
    border(border?: boolean): this;
    /** Set column padding. */
    padding(padding: number): this;
    /** Set column alignment. */
    align(direction: Direction): this;
    /** Get min column width. */
    getMinWidth(): number | undefined;
    /** Get max column width. */
    getMaxWidth(): number | undefined;
    /** Get column border. */
    getBorder(): boolean | undefined;
    /** Get column padding. */
    getPadding(): number | undefined;
    /** Get column alignment. */
    getAlign(): Direction | undefined;
}
//# sourceMappingURL=column.d.ts.map