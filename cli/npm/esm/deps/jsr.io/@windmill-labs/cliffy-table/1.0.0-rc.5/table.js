import { border } from "./border.js";
import { Cell } from "./cell.js";
import { Column } from "./column.js";
import { TableLayout } from "./_layout.js";
import { Row } from "./row.js";
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
export class Table extends Array {
    constructor() {
        super(...arguments);
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {
                indent: 0,
                border: false,
                maxColWidth: Infinity,
                minColWidth: 0,
                padding: 1,
                chars: { ...Table._chars },
                columns: [],
            }
        });
        Object.defineProperty(this, "headerRow", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
    }
    /**
     * Create a new table. If rows is a table, all rows and options of the table
     * will be copied to the new table.
     *
     * @param rows An array of rows or a table instance.
     */
    static from(rows) {
        const table = new this(...rows);
        if (rows instanceof Table) {
            table.options = { ...rows.options };
            table.headerRow = rows.headerRow ? Row.from(rows.headerRow) : undefined;
        }
        return table;
    }
    /**
     * Create a new table from an array of json objects. An object represents a
     * row and each property a column.
     *
     * @param rows Array of objects.
     */
    static fromJson(rows) {
        return new this().fromJson(rows);
    }
    /**
     * Set global default border characters.
     *
     * @param chars Border options.
     */
    static chars(chars) {
        Object.assign(this._chars, chars);
        return this;
    }
    /**
     * Write table or rows to stdout.
     *
     * @param rows Table or rows.
     */
    static render(rows) {
        Table.from(rows).render();
    }
    /**
     * Read data from an array of json objects. An object represents a
     * row and each property a column.
     *
     * @param rows Array of objects.
     */
    fromJson(rows) {
        this.header(Object.keys(rows[0]));
        this.body(rows.map((row) => Object.values(row)));
        return this;
    }
    /**
     * Set column options.
     *
     * @param columns An array of columns or column options.
     */
    columns(columns) {
        this.options.columns = columns.map((column) => column instanceof Column ? column : Column.from(column));
        return this;
    }
    /**
     * Set column options by index.
     *
     @param index   The column index.
     @param column  Column or column options.
     */
    column(index, column) {
        if (column instanceof Column) {
            this.options.columns[index] = column;
        }
        else if (this.options.columns[index]) {
            this.options.columns[index].options(column);
        }
        else {
            this.options.columns[index] = Column.from(column);
        }
        return this;
    }
    /**
     * Set table header.
     *
     * @param header Header row or cells.
     */
    header(header) {
        this.headerRow = header instanceof Row ? header : Row.from(header);
        return this;
    }
    /**
     * Set table body.
     *
     * @param rows Array of rows.
     */
    body(rows) {
        this.length = 0;
        this.push(...rows);
        return this;
    }
    /** Clone table recursively with header and options. */
    clone() {
        const table = new Table(...this.map((row) => row instanceof Row ? row.clone() : Row.from(row).clone()));
        table.options = { ...this.options };
        table.headerRow = this.headerRow?.clone();
        return table;
    }
    /** Generate table string. */
    toString() {
        return new TableLayout(this, this.options).toString();
    }
    /** Write table to stdout. */
    render() {
        console.log(this.toString());
        return this;
    }
    /**
     * Set max column width.
     *
     * @param width     Max column width.
     * @param override  Override existing value.
     */
    maxColWidth(width, override = true) {
        if (override || typeof this.options.maxColWidth === "undefined") {
            this.options.maxColWidth = width;
        }
        return this;
    }
    /**
     * Set min column width.
     *
     * @param width     Min column width.
     * @param override  Override existing value.
     */
    minColWidth(width, override = true) {
        if (override || typeof this.options.minColWidth === "undefined") {
            this.options.minColWidth = width;
        }
        return this;
    }
    /**
     * Set table indentation.
     *
     * @param width     Indent width.
     * @param override  Override existing value.
     */
    indent(width, override = true) {
        if (override || typeof this.options.indent === "undefined") {
            this.options.indent = width;
        }
        return this;
    }
    /**
     * Set cell padding.
     *
     * @param padding   Cell padding.
     * @param override  Override existing value.
     */
    padding(padding, override = true) {
        if (override || typeof this.options.padding === "undefined") {
            this.options.padding = padding;
        }
        return this;
    }
    /**
     * Enable/disable cell border.
     *
     * @param enable    Enable/disable cell border.
     * @param override  Override existing value.
     */
    border(enable = true, override = true) {
        if (override || typeof this.options.border === "undefined") {
            this.options.border = enable;
        }
        return this;
    }
    /**
     * Align table content.
     *
     * @param direction Align direction.
     * @param override  Override existing value.
     */
    align(direction, override = true) {
        if (override || typeof this.options.align === "undefined") {
            this.options.align = direction;
        }
        return this;
    }
    /**
     * Set border characters.
     *
     * @param chars Border options.
     */
    chars(chars) {
        Object.assign(this.options.chars, chars);
        return this;
    }
    /** Get table header. */
    getHeader() {
        return this.headerRow;
    }
    /** Get table body. */
    getBody() {
        return [...this];
    }
    /** Get max column width. */
    getMaxColWidth() {
        return this.options.maxColWidth;
    }
    /** Get min column width. */
    getMinColWidth() {
        return this.options.minColWidth;
    }
    /** Get table indentation. */
    getIndent() {
        return this.options.indent;
    }
    /** Get cell padding. */
    getPadding() {
        return this.options.padding;
    }
    /** Check if table has border. */
    getBorder() {
        return this.options.border === true;
    }
    /** Check if header row has border. */
    hasHeaderBorder() {
        const hasBorder = this.headerRow?.hasBorder();
        return hasBorder === true || (this.getBorder() && hasBorder !== false);
    }
    /** Check if table bordy has border. */
    hasBodyBorder() {
        return this.getBorder() ||
            this.options.columns.some((column) => column.getBorder()) ||
            this.some((row) => row instanceof Row
                ? row.hasBorder()
                : row.some((cell) => cell instanceof Cell ? cell.getBorder() : false));
    }
    /** Check if table header or body has border. */
    hasBorder() {
        return this.hasHeaderBorder() || this.hasBodyBorder();
    }
    /** Get table alignment. */
    getAlign() {
        return this.options.align ?? "left";
    }
    /** Get columns. */
    getColumns() {
        return this.options.columns;
    }
    /** Get column by column index. */
    getColumn(index) {
        return this.options.columns[index] ??= new Column();
    }
}
Object.defineProperty(Table, "_chars", {
    enumerable: true,
    configurable: true,
    writable: true,
    value: { ...border }
});
