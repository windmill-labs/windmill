import { type Border } from "./border.js";
import { type Direction } from "./cell.js";
import { Column, type ColumnOptions } from "./column.js";
import { type DataRow, Row, type RowType } from "./row.js";
/** Border characters settings. */
export type BorderOptions = Partial<Border>;
/** Table settings. */
export interface TableSettings {
    /** Table indentation. */
    indent: number;
    /** Enable/disable border on all cells. */
    border: boolean;
    /** Set min column width. */
    minColWidth: number | Array<number>;
    /** Set max column width. */
    maxColWidth: number | Array<number>;
    /** Set cell padding. */
    padding: number | Array<number>;
    /** Set table characters. */
    chars: Border;
    /** Set cell content alignment. */
    align?: Direction;
    /** Set column options. */
    columns: Array<Column>;
}
/** Table type. */
export type TableType<TRow extends RowType = RowType> = Array<TRow> | Table<TRow>;
/**
 * Table representation.
 *
 * ```ts
 * import { Row, Table } from "./mod.ts";
 *
 * new Table()
 *   .header(new Row("Name", "Date", "City", "Country").border())
 *   .body([
 *     ["Baxter Herman", "Oct 1, 2020", "Harderwijk", "Slovenia"],
 *     ["Jescie Wolfe", "Dec 4, 2020", "Alto Hospicio", "Japan"],
 *     ["Allegra Cleveland", "Apr 16, 2020", "Avernas-le-Bauduin", "Samoa"],
 *     ["Aretha Gamble", "Feb 22, 2021", "Honolulu", "Georgia"],
 *   ])
 *   .render();
 * ```
 */
export declare class Table<TRow extends RowType = RowType> extends Array<TRow> {
    protected static _chars: Border;
    protected options: TableSettings;
    private headerRow?;
    /**
     * Create a new table. If rows is a table, all rows and options of the table
     * will be copied to the new table.
     *
     * @param rows An array of rows or a table instance.
     */
    static from<TRow extends RowType>(rows: TableType<TRow>): Table<TRow>;
    /**
     * Create a new table from an array of json objects. An object represents a
     * row and each property a column.
     *
     * @param rows Array of objects.
     */
    static fromJson(rows: Array<DataRow>): Table;
    /**
     * Set global default border characters.
     *
     * @param chars Border options.
     */
    static chars(chars: BorderOptions): typeof Table;
    /**
     * Write table or rows to stdout.
     *
     * @param rows Table or rows.
     */
    static render<TRow extends RowType>(rows: TableType<TRow>): void;
    /**
     * Read data from an array of json objects. An object represents a
     * row and each property a column.
     *
     * @param rows Array of objects.
     */
    fromJson(rows: Array<DataRow>): this;
    /**
     * Set column options.
     *
     * @param columns An array of columns or column options.
     */
    columns(columns: Array<Column | ColumnOptions>): this;
    /**
     * Set column options by index.
     *
     @param index   The column index.
     @param column  Column or column options.
     */
    column(index: number, column: Column | ColumnOptions): this;
    /**
     * Set table header.
     *
     * @param header Header row or cells.
     */
    header(header: RowType): this;
    /**
     * Set table body.
     *
     * @param rows Array of rows.
     */
    body(rows: Array<TRow>): this;
    /** Clone table recursively with header and options. */
    clone(): Table;
    /** Generate table string. */
    toString(): string;
    /** Write table to stdout. */
    render(): this;
    /**
     * Set max column width.
     *
     * @param width     Max column width.
     * @param override  Override existing value.
     */
    maxColWidth(width: number | Array<number>, override?: boolean): this;
    /**
     * Set min column width.
     *
     * @param width     Min column width.
     * @param override  Override existing value.
     */
    minColWidth(width: number | Array<number>, override?: boolean): this;
    /**
     * Set table indentation.
     *
     * @param width     Indent width.
     * @param override  Override existing value.
     */
    indent(width: number, override?: boolean): this;
    /**
     * Set cell padding.
     *
     * @param padding   Cell padding.
     * @param override  Override existing value.
     */
    padding(padding: number | Array<number>, override?: boolean): this;
    /**
     * Enable/disable cell border.
     *
     * @param enable    Enable/disable cell border.
     * @param override  Override existing value.
     */
    border(enable?: boolean, override?: boolean): this;
    /**
     * Align table content.
     *
     * @param direction Align direction.
     * @param override  Override existing value.
     */
    align(direction: Direction, override?: boolean): this;
    /**
     * Set border characters.
     *
     * @param chars Border options.
     */
    chars(chars: BorderOptions): this;
    /** Get table header. */
    getHeader(): Row | undefined;
    /** Get table body. */
    getBody(): Array<TRow>;
    /** Get max column width. */
    getMaxColWidth(): number | Array<number>;
    /** Get min column width. */
    getMinColWidth(): number | Array<number>;
    /** Get table indentation. */
    getIndent(): number;
    /** Get cell padding. */
    getPadding(): number | Array<number>;
    /** Check if table has border. */
    getBorder(): boolean;
    /** Check if header row has border. */
    hasHeaderBorder(): boolean;
    /** Check if table bordy has border. */
    hasBodyBorder(): boolean;
    /** Check if table header or body has border. */
    hasBorder(): boolean;
    /** Get table alignment. */
    getAlign(): Direction;
    /** Get columns. */
    getColumns(): Array<Column>;
    /** Get column by column index. */
    getColumn(index: number): Column;
}
/** @deprecated Use `BorderOptions` instead. */
export type IBorderOptions = BorderOptions;
/** @deprecated Use `TableType` instead. */
export type ITable = TableType;
//# sourceMappingURL=table.d.ts.map