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
export class Cell {
  protected options: CellOptions = {};

  /** Get cell length. */
  public get length(): number {
    return this.toString().length;
  }

  /**
   * Any unterminated ANSI formatting overflowed from previous lines of a
   * multi-line cell.
   */
  public get unclosedAnsiRuns(): string {
    return this.options.unclosedAnsiRuns ?? "";
  }
  public set unclosedAnsiRuns(val: string) {
    this.options.unclosedAnsiRuns = val;
  }

  /**
   * Create a new cell. If value is a cell, the value and all options of the cell
   * will be copied to the new cell.
   *
   * @param value Cell or cell value.
   */
  public static from(value: CellType): Cell {
    let cell: Cell;
    if (value instanceof Cell) {
      cell = new this(value.getValue());
      cell.options = { ...value.options };
    } else {
      cell = new this(value);
    }
    return cell;
  }

  /**
   * Cell constructor.
   *
   * @param value Cell value.
   */
  public constructor(private value: CellValue) {}

  /** Get cell string value. */
  public toString(): string {
    return this.value.toString();
  }

  /** Get cell value. */
  public getValue(): CellValue {
    return this.value;
  }

  /**
   * Set cell value.
   *
   * @param value Cell or cell value.
   */
  public setValue(value: CellValue): this {
    this.value = value;
    return this;
  }

  /**
   * Clone cell with all options.
   *
   * @param value Cell or cell value.
   */
  public clone(value?: CellValue): Cell {
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
  public border(enable = true, override = true): this {
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
  public colSpan(span: number, override = true): this {
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
  public rowSpan(span: number, override = true): this {
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
  public align(direction: Direction, override = true): this {
    if (override || typeof this.options.align === "undefined") {
      this.options.align = direction;
    }
    return this;
  }

  /**
   * Getter:
   */

  /** Check if cell has border. */
  public getBorder(): boolean {
    return this.options.border === true;
  }

  /** Get col span. */
  public getColSpan(): number {
    return typeof this.options.colSpan === "number" && this.options.colSpan > 0
      ? this.options.colSpan
      : 1;
  }

  /** Get row span. */
  public getRowSpan(): number {
    return typeof this.options.rowSpan === "number" && this.options.rowSpan > 0
      ? this.options.rowSpan
      : 1;
  }

  /** Get row span. */
  public getAlign(): Direction {
    return this.options.align ?? "left";
  }
}

/** @deprecated Use `CellType` instead. */
export type ICell = CellType;
