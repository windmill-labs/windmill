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
    constructor() {
        Object.defineProperty(this, "opts", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: {}
        });
    }
    /**
     * Create a new cell from column options or an existing column.
     * @param options
     */
    static from(options) {
        const opts = options instanceof Column ? options.opts : options;
        return new Column().options(opts);
    }
    /** Set column options. */
    options(options) {
        Object.assign(this.opts, options);
        return this;
    }
    /** Set min column width. */
    minWidth(width) {
        this.opts.minWidth = width;
        return this;
    }
    /** Set max column width. */
    maxWidth(width) {
        this.opts.maxWidth = width;
        return this;
    }
    /** Set column border. */
    border(border = true) {
        this.opts.border = border;
        return this;
    }
    /** Set column padding. */
    padding(padding) {
        this.opts.padding = padding;
        return this;
    }
    /** Set column alignment. */
    align(direction) {
        this.opts.align = direction;
        return this;
    }
    /** Get min column width. */
    getMinWidth() {
        return this.opts.minWidth;
    }
    /** Get max column width. */
    getMaxWidth() {
        return this.opts.maxWidth;
    }
    /** Get column border. */
    getBorder() {
        return this.opts.border;
    }
    /** Get column padding. */
    getPadding() {
        return this.opts.padding;
    }
    /** Get column alignment. */
    getAlign() {
        return this.opts.align;
    }
}
