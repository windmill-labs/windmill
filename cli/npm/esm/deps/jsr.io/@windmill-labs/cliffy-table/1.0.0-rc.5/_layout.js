var __classPrivateFieldGet = (this && this.__classPrivateFieldGet) || function (receiver, state, kind, f) {
    if (kind === "a" && !f) throw new TypeError("Private accessor was defined without a getter");
    if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver)) throw new TypeError("Cannot read private member from an object whose class did not declare it");
    return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
};
var _TableLayout_instances, _TableLayout_getRows;
import { Cell } from "./cell.js";
import { consumeChars, consumeWords } from "./consume_words.js";
import { Row } from "./row.js";
import { getUnclosedAnsiRuns, longest, strLength } from "./_utils.js";
/** Table layout renderer. */
export class TableLayout {
    /**
     * Table layout constructor.
     * @param table   Table instance.
     * @param options Render options.
     */
    constructor(table, options) {
        _TableLayout_instances.add(this);
        Object.defineProperty(this, "table", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: table
        });
        Object.defineProperty(this, "options", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: options
        });
    }
    /** Generate table string. */
    toString() {
        const opts = this.createLayout();
        return opts.rows.length ? this.renderRows(opts) : "";
    }
    /**
     * Generates table layout including row and col span, converts all none
     * Cell/Row values to Cells and Rows and returns the layout rendering
     * settings.
     */
    createLayout() {
        Object.keys(this.options.chars).forEach((key) => {
            if (typeof this.options.chars[key] !== "string") {
                this.options.chars[key] = "";
            }
        });
        const hasBodyBorder = this.table.getBorder() ||
            this.table.hasBodyBorder();
        const hasHeaderBorder = this.table.hasHeaderBorder();
        const hasBorder = hasHeaderBorder || hasBodyBorder;
        const rows = __classPrivateFieldGet(this, _TableLayout_instances, "m", _TableLayout_getRows).call(this);
        const columns = Math.max(...rows.map((row) => row.length));
        for (let rowIndex = 0; rowIndex < rows.length; rowIndex++) {
            const row = rows[rowIndex];
            const length = row.length;
            if (length < columns) {
                const diff = columns - length;
                for (let i = 0; i < diff; i++) {
                    row.push(this.createCell(null, row, rowIndex, length + i));
                }
            }
        }
        const padding = [];
        const width = [];
        for (let colIndex = 0; colIndex < columns; colIndex++) {
            const column = this.options.columns.at(colIndex);
            const minColWidth = column?.getMinWidth() ??
                (Array.isArray(this.options.minColWidth)
                    ? this.options.minColWidth[colIndex]
                    : this.options.minColWidth);
            const maxColWidth = column?.getMaxWidth() ??
                (Array.isArray(this.options.maxColWidth)
                    ? this.options.maxColWidth[colIndex]
                    : this.options.maxColWidth);
            const colWidth = longest(colIndex, rows, maxColWidth);
            width[colIndex] = Math.min(maxColWidth, Math.max(minColWidth, colWidth));
            padding[colIndex] = column?.getPadding() ??
                (Array.isArray(this.options.padding)
                    ? this.options.padding[colIndex]
                    : this.options.padding);
        }
        return {
            padding,
            width,
            rows,
            columns,
            hasBorder,
            hasBodyBorder,
            hasHeaderBorder,
        };
    }
    /**
     * Fills rows and cols by specified row/col span with a reference of the
     * original cell.
     */
    spanRows(rows) {
        const rowSpan = [];
        let colSpan = 1;
        let rowIndex = -1;
        while (true) {
            rowIndex++;
            if (rowIndex === rows.length && rowSpan.every((span) => span === 1)) {
                break;
            }
            const row = rows[rowIndex] = this.createRow(rows[rowIndex] || []);
            let colIndex = -1;
            while (true) {
                colIndex++;
                if (colIndex === row.length &&
                    colIndex === rowSpan.length && colSpan === 1) {
                    break;
                }
                if (colSpan > 1) {
                    colSpan--;
                    rowSpan[colIndex] = rowSpan[colIndex - 1];
                    row.splice(colIndex, this.getDeleteCount(rows, rowIndex, colIndex), row[colIndex - 1]);
                    continue;
                }
                if (rowSpan[colIndex] > 1) {
                    rowSpan[colIndex]--;
                    rows[rowIndex].splice(colIndex, this.getDeleteCount(rows, rowIndex, colIndex), rows[rowIndex - 1][colIndex]);
                    continue;
                }
                const cell = row[colIndex] = this.createCell(row[colIndex] || null, row, rowIndex, colIndex);
                colSpan = cell.getColSpan();
                rowSpan[colIndex] = cell.getRowSpan();
            }
        }
        return rows;
    }
    getDeleteCount(rows, rowIndex, colIndex) {
        return colIndex <= rows[rowIndex].length - 1 &&
            typeof rows[rowIndex][colIndex] === "undefined"
            ? 1
            : 0;
    }
    /**
     * Create a new row from existing row or cell array.
     * @param row Original row.
     */
    createRow(row) {
        return Row.from(row)
            .border(this.table.getBorder(), false)
            .align(this.table.getAlign(), false);
    }
    /**
     * Create a new cell from existing cell or cell value.
     *
     * @param cell      Original cell.
     * @param row       Parent row.
     * @param rowIndex  The row index of the cell.
     * @param colIndex  The column index of the cell.
     */
    createCell(cell, row, rowIndex, colIndex) {
        const column = this.options.columns.at(colIndex);
        const isHeaderRow = this.isHeaderRow(rowIndex);
        return Cell.from(cell ?? "")
            .border((isHeaderRow ? null : column?.getBorder()) ?? row.getBorder(), false)
            .align((isHeaderRow ? null : column?.getAlign()) ?? row.getAlign(), false);
    }
    isHeaderRow(rowIndex) {
        return rowIndex === 0 && this.table.getHeader() !== undefined;
    }
    /**
     * Render table layout.
     * @param opts Render options.
     */
    renderRows(opts) {
        let result = "";
        const rowSpan = new Array(opts.columns).fill(1);
        for (let rowIndex = 0; rowIndex < opts.rows.length; rowIndex++) {
            result += this.renderRow(rowSpan, rowIndex, opts);
        }
        return result.slice(0, -1);
    }
    /**
     * Render row.
     * @param rowSpan     Current row span.
     * @param rowIndex    Current row index.
     * @param opts        Render options.
     * @param isMultiline Is multiline row.
     */
    renderRow(rowSpan, rowIndex, opts, isMultiline) {
        const row = opts.rows[rowIndex];
        const prevRow = opts.rows[rowIndex - 1];
        const nextRow = opts.rows[rowIndex + 1];
        let result = "";
        let colSpan = 1;
        // border top row
        if (!isMultiline && rowIndex === 0 && row.hasBorder()) {
            result += this.renderBorderRow(undefined, row, rowSpan, opts);
        }
        let isMultilineRow = false;
        result += " ".repeat(this.options.indent || 0);
        for (let colIndex = 0; colIndex < opts.columns; colIndex++) {
            if (colSpan > 1) {
                colSpan--;
                rowSpan[colIndex] = rowSpan[colIndex - 1];
                continue;
            }
            result += this.renderCell(colIndex, row, opts);
            if (rowSpan[colIndex] > 1) {
                if (!isMultiline) {
                    rowSpan[colIndex]--;
                }
            }
            else if (!prevRow || prevRow[colIndex] !== row[colIndex]) {
                rowSpan[colIndex] = row[colIndex].getRowSpan();
            }
            colSpan = row[colIndex].getColSpan();
            if (rowSpan[colIndex] === 1 && row[colIndex].length) {
                isMultilineRow = true;
            }
        }
        if (opts.columns > 0) {
            if (row[opts.columns - 1].getBorder()) {
                result += this.options.chars.right;
            }
            else if (opts.hasBorder) {
                result += " ";
            }
        }
        result += "\n";
        if (isMultilineRow) { // skip border
            return result + this.renderRow(rowSpan, rowIndex, opts, isMultilineRow);
        }
        // border mid row
        if ((opts.rows.length > 1) &&
            ((rowIndex === 0 && opts.hasHeaderBorder) ||
                (rowIndex < opts.rows.length - 1 && opts.hasBodyBorder))) {
            result += this.renderBorderRow(row, nextRow, rowSpan, opts);
        }
        // border bottom row
        if (rowIndex === opts.rows.length - 1 && row.hasBorder()) {
            result += this.renderBorderRow(row, undefined, rowSpan, opts);
        }
        return result;
    }
    /**
     * Render cell.
     * @param colIndex  Current col index.
     * @param row       Current row.
     * @param opts      Render options.
     * @param noBorder  Disable border.
     */
    renderCell(colIndex, row, opts, noBorder) {
        let result = "";
        const prevCell = row[colIndex - 1];
        const cell = row[colIndex];
        if (!noBorder) {
            if (colIndex === 0) {
                if (cell.getBorder()) {
                    result += this.options.chars.left;
                }
                else if (opts.hasBorder) {
                    result += " ";
                }
            }
            else {
                if (cell.getBorder() || prevCell?.getBorder()) {
                    result += this.options.chars.middle;
                }
                else if (opts.hasBorder) {
                    result += " ";
                }
            }
        }
        let maxLength = opts.width[colIndex];
        const colSpan = cell.getColSpan();
        if (colSpan > 1) {
            for (let o = 1; o < colSpan; o++) {
                // add padding and with of next cell
                maxLength += opts.width[colIndex + o] + opts.padding[colIndex + o];
                if (opts.hasBorder) {
                    // add padding again and border with
                    maxLength += opts.padding[colIndex + o] + 1;
                }
            }
        }
        const { current, next } = this.renderCellValue(cell, maxLength);
        row[colIndex].setValue(next);
        if (opts.hasBorder) {
            result += " ".repeat(opts.padding[colIndex]);
        }
        result += current;
        if (opts.hasBorder || colIndex < opts.columns - 1) {
            result += " ".repeat(opts.padding[colIndex]);
        }
        return result;
    }
    /**
     * Render specified length of cell. Returns the rendered value and a new cell
     * with the rest value.
     * @param cell      Cell to render.
     * @param maxLength Max length of content to render.
     */
    renderCellValue(cell, maxLength) {
        const length = Math.min(maxLength, strLength(cell.toString()));
        let words = consumeWords(length, cell.toString());
        // break word if word is longer than max length
        const breakWord = strLength(words) > length;
        if (breakWord) {
            words = consumeChars(length, words);
        }
        // get next content and remove leading space if breakWord is not true
        // calculate from words.length _before_ any handling of unclosed ANSI codes
        const next = cell.toString().slice(words.length + (breakWord ? 0 : 1));
        words = cell.unclosedAnsiRuns + words;
        const { currentSuffix, nextPrefix } = getUnclosedAnsiRuns(words);
        words += currentSuffix;
        cell.unclosedAnsiRuns = nextPrefix;
        const fillLength = maxLength - strLength(words);
        // Align content
        const align = cell.getAlign();
        let current;
        if (fillLength === 0) {
            current = words;
        }
        else if (align === "left") {
            current = words + " ".repeat(fillLength);
        }
        else if (align === "center") {
            current = " ".repeat(Math.floor(fillLength / 2)) + words +
                " ".repeat(Math.ceil(fillLength / 2));
        }
        else if (align === "right") {
            current = " ".repeat(fillLength) + words;
        }
        else {
            throw new Error("Unknown direction: " + align);
        }
        return { current, next };
    }
    /**
     * Render border row.
     * @param prevRow Previous row.
     * @param nextRow Next row.
     * @param rowSpan Current row span.
     * @param opts    Render options.
     */
    renderBorderRow(prevRow, nextRow, rowSpan, opts) {
        let result = "";
        let colSpan = 1;
        for (let colIndex = 0; colIndex < opts.columns; colIndex++) {
            if (rowSpan[colIndex] > 1) {
                if (!nextRow) {
                    throw new Error("invalid layout");
                }
                if (colSpan > 1) {
                    colSpan--;
                    continue;
                }
            }
            result += this.renderBorderCell(colIndex, prevRow, nextRow, rowSpan, opts);
            colSpan = nextRow?.[colIndex].getColSpan() ?? 1;
        }
        return result.length ? " ".repeat(this.options.indent) + result + "\n" : "";
    }
    /**
     * Render border cell.
     * @param colIndex  Current index.
     * @param prevRow   Previous row.
     * @param nextRow   Next row.
     * @param rowSpan   Current row span.
     * @param opts      Render options.
     */
    renderBorderCell(colIndex, prevRow, nextRow, rowSpan, opts) {
        // a1 | b1
        // -------
        // a2 | b2
        const a1 = prevRow?.[colIndex - 1];
        const a2 = nextRow?.[colIndex - 1];
        const b1 = prevRow?.[colIndex];
        const b2 = nextRow?.[colIndex];
        const a1Border = !!a1?.getBorder();
        const a2Border = !!a2?.getBorder();
        const b1Border = !!b1?.getBorder();
        const b2Border = !!b2?.getBorder();
        const hasColSpan = (cell) => (cell?.getColSpan() ?? 1) > 1;
        const hasRowSpan = (cell) => (cell?.getRowSpan() ?? 1) > 1;
        let result = "";
        if (colIndex === 0) {
            if (rowSpan[colIndex] > 1) {
                if (b1Border) {
                    result += this.options.chars.left;
                }
                else {
                    result += " ";
                }
            }
            else if (b1Border && b2Border) {
                result += this.options.chars.leftMid;
            }
            else if (b1Border) {
                result += this.options.chars.bottomLeft;
            }
            else if (b2Border) {
                result += this.options.chars.topLeft;
            }
            else {
                result += " ";
            }
        }
        else if (colIndex < opts.columns) {
            if ((a1Border && b2Border) || (b1Border && a2Border)) {
                const a1ColSpan = hasColSpan(a1);
                const a2ColSpan = hasColSpan(a2);
                const b1ColSpan = hasColSpan(b1);
                const b2ColSpan = hasColSpan(b2);
                const a1RowSpan = hasRowSpan(a1);
                const a2RowSpan = hasRowSpan(a2);
                const b1RowSpan = hasRowSpan(b1);
                const b2RowSpan = hasRowSpan(b2);
                const hasAllBorder = a1Border && b2Border && b1Border && a2Border;
                const hasAllRowSpan = a1RowSpan && b1RowSpan && a2RowSpan && b2RowSpan;
                const hasAllColSpan = a1ColSpan && b1ColSpan && a2ColSpan && b2ColSpan;
                if (hasAllRowSpan && hasAllBorder) {
                    result += this.options.chars.middle;
                }
                else if (hasAllColSpan && hasAllBorder && a1 === b1 && a2 === b2) {
                    result += this.options.chars.mid;
                }
                else if (a1ColSpan && b1ColSpan && a1 === b1) {
                    result += this.options.chars.topMid;
                }
                else if (a2ColSpan && b2ColSpan && a2 === b2) {
                    result += this.options.chars.bottomMid;
                }
                else if (a1RowSpan && a2RowSpan && a1 === a2) {
                    result += this.options.chars.leftMid;
                }
                else if (b1RowSpan && b2RowSpan && b1 === b2) {
                    result += this.options.chars.rightMid;
                }
                else {
                    result += this.options.chars.midMid;
                }
            }
            else if (a1Border && b1Border) {
                if (hasColSpan(a1) && hasColSpan(b1) && a1 === b1) {
                    result += this.options.chars.bottom;
                }
                else {
                    result += this.options.chars.bottomMid;
                }
            }
            else if (b1Border && b2Border) {
                if (rowSpan[colIndex] > 1) {
                    result += this.options.chars.left;
                }
                else {
                    result += this.options.chars.leftMid;
                }
            }
            else if (b2Border && a2Border) {
                if (hasColSpan(a2) && hasColSpan(b2) && a2 === b2) {
                    result += this.options.chars.top;
                }
                else {
                    result += this.options.chars.topMid;
                }
            }
            else if (a1Border && a2Border) {
                if (hasRowSpan(a1) && a1 === a2) {
                    result += this.options.chars.right;
                }
                else {
                    result += this.options.chars.rightMid;
                }
            }
            else if (a1Border) {
                result += this.options.chars.bottomRight;
            }
            else if (b1Border) {
                result += this.options.chars.bottomLeft;
            }
            else if (a2Border) {
                result += this.options.chars.topRight;
            }
            else if (b2Border) {
                result += this.options.chars.topLeft;
            }
            else {
                result += " ";
            }
        }
        const length = opts.padding[colIndex] + opts.width[colIndex] +
            opts.padding[colIndex];
        if (rowSpan[colIndex] > 1 && nextRow) {
            result += this.renderCell(colIndex, nextRow, opts, true);
            if (nextRow[colIndex] === nextRow[nextRow.length - 1]) {
                if (b1Border) {
                    result += this.options.chars.right;
                }
                else {
                    result += " ";
                }
                return result;
            }
        }
        else if (b1Border && b2Border) {
            result += this.options.chars.mid.repeat(length);
        }
        else if (b1Border) {
            result += this.options.chars.bottom.repeat(length);
        }
        else if (b2Border) {
            result += this.options.chars.top.repeat(length);
        }
        else {
            result += " ".repeat(length);
        }
        if (colIndex === opts.columns - 1) {
            if (b1Border && b2Border) {
                result += this.options.chars.rightMid;
            }
            else if (b1Border) {
                result += this.options.chars.bottomRight;
            }
            else if (b2Border) {
                result += this.options.chars.topRight;
            }
            else {
                result += " ";
            }
        }
        return result;
    }
}
_TableLayout_instances = new WeakSet(), _TableLayout_getRows = function _TableLayout_getRows() {
    const header = this.table.getHeader();
    const rows = header ? [header, ...this.table] : this.table.slice();
    const hasSpan = rows.some((row) => row.some((cell) => cell instanceof Cell && (cell.getColSpan() > 1 || cell.getRowSpan() > 1)));
    if (hasSpan) {
        return this.spanRows(rows);
    }
    return rows.map((row, rowIndex) => {
        const newRow = this.createRow(row);
        for (let colIndex = 0; colIndex < row.length; colIndex++) {
            newRow[colIndex] = this.createCell(row[colIndex], newRow, rowIndex, colIndex);
        }
        return newRow;
    });
};
