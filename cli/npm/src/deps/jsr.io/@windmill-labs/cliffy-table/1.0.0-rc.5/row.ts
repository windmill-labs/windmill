import { Cell, type CellType, type Direction } from "./cell.js";

/** Allowed row type. */
export type RowType<
  T extends CellType | undefined = CellType | undefined,
> =
  | Array<T>
  | Row<T>;

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
export class Row<
  T extends CellType | undefined = CellType | undefined,
> extends Array<T> {
  protected options: RowOptions = {};

  /**
   * Create a new row. If cells is a row, all cells and options of the row will
   * be copied to the new row.
   *
   * @param cells Cells or row.
   */
  public static from<T extends CellType | undefined>(
    cells: RowType<T>,
  ): Row<T> {
    const row = new this(...cells);
    if (cells instanceof Row) {
      row.options = { ...(cells as Row).options };
    }
    return row;
  }

  /** Clone row recursively with all options. */
  public clone(): Row {
    const row = new Row(
      ...this.map((cell: T) => cell instanceof Cell ? cell.clone() : cell),
    );
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
  public border(enable = true, override = true): this {
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
  public align(direction: Direction, override = true): this {
    if (override || typeof this.options.align === "undefined") {
      this.options.align = direction;
    }
    return this;
  }

  /**
   * Getter:
   */

  /** Check if row has border. */
  public getBorder(): boolean {
    return this.options.border === true;
  }

  /** Check if row or any child cell has border. */
  public hasBorder(): boolean {
    return this.getBorder() ||
      this.some((cell) => cell instanceof Cell && cell.getBorder());
  }

  /** Get row alignment. */
  public getAlign(): Direction {
    return this.options.align ?? "left";
  }
}

/** @deprecated Use `RowType` instead. */
export type IRow = RowType;

/** @deprecated Use `DataRow` instead. */
export type IDataRow = DataRow;
