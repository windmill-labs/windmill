import { Cell } from "./cell.js";
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
export class Row extends Array {
    constructor() {
        super(...arguments);
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {}
        });
    }
    /**
     * Create a new row. If cells is a row, all cells and options of the row will
     * be copied to the new row.
     *
     * @param cells Cells or row.
     */
    static from(cells) {
        const row = new this(...cells);
        if (cells instanceof Row) {
            row.options = { ...cells.options };
        }
        return row;
    }
    /** Clone row recursively with all options. */
    clone() {
        const row = new Row(...this.map((cell) => cell instanceof Cell ? cell.clone() : cell));
        row.options = { ...this.options };
        return row;
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
     * Align row content.
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
    /** Check if row has border. */
    getBorder() {
        return this.options.border === true;
    }
    /** Check if row or any child cell has border. */
    hasBorder() {
        return this.getBorder() ||
            this.some((cell) => cell instanceof Cell && cell.getBorder());
    }
    /** Get row alignment. */
    getAlign() {
        return this.options.align ?? "left";
    }
}
