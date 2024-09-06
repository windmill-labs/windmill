import { type CellType, type Direction } from "./cell.js";
/** Allowed row type. */
export type RowType<T extends CellType | undefined = CellType | undefined> = Array<T> | Row<T>;
/** Json row. */
export type DataRow = Record<string, string | number>;
/** Row options. */
interface RowOptions {
    indent?: number;
    border?: boolean;
    align?: Direction;
}
/**
 * Row representation.
 *
 * Can be used to customize a single row.
 *
 * ```ts
 * import { Row, Table } from "./mod.ts";
 *
 * new Table()
 *   .body([
 *     new Row("Foo", "Bar").align("right"),
 *     ["Beep", "Boop"],
 *   ])
 *   .render();
 * ```
 */
export declare class Row<T extends CellType | undefined = CellType | undefined> extends Array<T> {
    protected options: RowOptions;
    /**
     * Create a new row. If cells is a row, all cells and options of the row will
     * be copied to the new row.
     *
     * @param cells Cells or row.
     */
    static from<T extends CellType | undefined>(cells: RowType<T>): Row<T>;
    /** Clone row recursively with all options. */
    clone(): Row;
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
     * Align row content.
     *
     * @param direction Align direction.
     * @param override  Override existing value.
     */
    align(direction: Direction, override?: boolean): this;
    /**
     * Getter:
     */
    /** Check if row has border. */
    getBorder(): boolean;
    /** Check if row or any child cell has border. */
    hasBorder(): boolean;
    /** Get row alignment. */
    getAlign(): Direction;
}
/** @deprecated Use `RowType` instead. */
export type IRow = RowType;
/** @deprecated Use `DataRow` instead. */
export type IDataRow = DataRow;
export {};
//# sourceMappingURL=row.d.ts.map