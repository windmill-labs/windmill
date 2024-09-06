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
export class Cell {
    /** Get cell length. */
    get length() {
        return this.toString().length;
    }
    /**
     * Any unterminated ANSI formatting overflowed from previous lines of a
     * multi-line cell.
     */
    get unclosedAnsiRuns() {
        return this.options.unclosedAnsiRuns ?? "";
    }
    set unclosedAnsiRuns(val) {
        this.options.unclosedAnsiRuns = val;
    }
    /**
     * Create a new cell. If value is a cell, the value and all options of the cell
     * will be copied to the new cell.
     *
     * @param value Cell or cell value.
     */
    static from(value) {
        let cell;
        if (value instanceof Cell) {
            cell = new this(value.getValue());
            cell.options = { ...value.options };
        }
        else {
            cell = new this(value);
        }
        return cell;
    }
    /**
     * Cell constructor.
     *
     * @param value Cell value.
     */
    constructor(value) {
        Object.defineProperty(this, "value", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: value
        });
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {}
        });
    }
    /** Get cell string value. */
    toString() {
        return this.value.toString();
    }
    /** Get cell value. */
    getValue() {
        return this.value;
    }
    /**
     * Set cell value.
     *
     * @param value Cell or cell value.
     */
    setValue(value) {
        this.value = value;
        return this;
    }
    /**
     * Clone cell with all options.
     *
     * @param value Cell or cell value.
     */
    clone(value) {
        return Cell.from(value ?? this);
    }
    /**
     * Setter:
     */
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
    colSpan(span, override = true) {
        if (override || typeof this.options.colSpan === "undefined") {
            this.options.colSpan = span;
        }
        return this;
    }
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
    rowSpan(span, override = true) {
        if (override || typeof this.options.rowSpan === "undefined") {
            this.options.rowSpan = span;
        }
        return this;
    }
    /**
     * Align cell content.
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
     * Getter:
     */
    /** Check if cell has border. */
    getBorder() {
        return this.options.border === true;
    }
    /** Get col span. */
    getColSpan() {
        return typeof this.options.colSpan === "number" && this.options.colSpan > 0
            ? this.options.colSpan
            : 1;
    }
    /** Get row span. */
    getRowSpan() {
        return typeof this.options.rowSpan === "number" && this.options.rowSpan > 0
            ? this.options.rowSpan
            : 1;
    }
    /** Get row span. */
    getAlign() {
        return this.options.align ?? "left";
    }
}
