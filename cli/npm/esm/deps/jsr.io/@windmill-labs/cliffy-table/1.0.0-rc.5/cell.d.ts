/** Allowed cell value type. */
export type CellValue = number | string;
/** Allowed cell type. */
export type CellType = CellValue | Cell;
/** Cell alignment direction. */
export type Direction = "left" | "right" | "center";
/** Cell options. */
interface CellOptions {
    /** Enable/disable cell border. */
    border?: boolean;
    /** Set coll span. */
    colSpan?: number;
    /** Set row span. */
    rowSpan?: number;
    /** Cell cell alignment direction. */
    align?: Direction;
    /**
     * Any unterminated ANSI formatting overflowed from previous lines of a
     * multi-line cell.
     */
    unclosedAnsiRuns?: string;
}
/**
 * Cell representation.
 *
 * Can be used to customize a single cell.
 *
 * ```ts
 * import { Cell, Table } from "./mod.ts";
 *
 * new Table()
 *   .body([
 *     ["Foo", new Cell("Bar").align("right")],
 *     ["Beep", "Boop"],
 *   ])
 *   .render();
 * ```
 */
export declare class Cell {
    private value;
    protected options: CellOptions;
    /** Get cell length. */
    get length(): number;
    /**
     * Any unterminated ANSI formatting overflowed from previous lines of a
     * multi-line cell.
     */
    get unclosedAnsiRuns(): string;
    set unclosedAnsiRuns(val: string);
    /**
     * Create a new cell. If value is a cell, the value and all options of the cell
     * will be copied to the new cell.
     *
     * @param value Cell or cell value.
     */
    static from(value: CellType): Cell;
    /**
     * Cell constructor.
     *
     * @param value Cell value.
     */
    constructor(value: CellValue);
    /** Get cell string value. */
    toString(): string;
    /** Get cell value. */
    getValue(): CellValue;
    /**
     * Set cell value.
     *
     * @param value Cell or cell value.
     */
    setValue(value: CellValue): this;
    /**
     * Clone cell with all options.
     *
     * @param value Cell or cell value.
     */
    clone(value?: CellValue): Cell;
    /**
     * Setter:
     */
    /**
     * Enable/disable cell border.
     *
     * @param enable    Enable/disable cell border.
     * @param override  Override existing value.
     */
    border(enable?: boolean, override?: boolean): this;
    /**
     * Set col span.
     *
     * ```ts
     * import { Cell, Table } from "./mod.ts";
     *
     * new Table()
     *   .body([
     *     [
     *       new Cell("Row 1 & 2 Column 1").rowSpan(2),
     *       "Row 1 Column 2",
     *       "Row 1 Column 3",
     *     ],
     *     [new Cell("Row 2 Column 2 & 3").colSpan(2)],
     *   ])
     *   .border()
     *   .render();
     * ```
     *
     * @param span      Number of cols to span.
     * @param override  Override existing value.
     */
    colSpan(span: number, override?: boolean): this;
    /**
     * Set row span.
     *
     * ```ts
     * import { Cell, Table } from "./mod.ts";
     *
     * new Table()
     *   .body([
     *     [
     *       new Cell("Row 1 & 2 Column 1").rowSpan(2),
     *       "Row 1 Column 2",
     *       "Row 1 Column 3",
     *     ],
     *     [new Cell("Row 2 Column 2 & 3").colSpan(2)],
     *   ])
     *   .border()
     *   .render();
     * ```
     *
     * @param span      Number of rows to span.
     * @param override  Override existing value.
     */
    rowSpan(span: number, override?: boolean): this;
    /**
     * Align cell content.
     *
     * @param direction Align direction.
     * @param override  Override existing value.
     */
    align(direction: Direction, override?: boolean): this;
    /**
     * Getter:
     */
    /** Check if cell has border. */
    getBorder(): boolean;
    /** Get col span. */
    getColSpan(): number;
    /** Get row span. */
    getRowSpan(): number;
    /** Get row span. */
    getAlign(): Direction;
}
/** @deprecated Use `CellType` instead. */
export type ICell = CellType;
export {};
//# sourceMappingURL=cell.d.ts.map