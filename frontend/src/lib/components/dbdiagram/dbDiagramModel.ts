import type { DBSchema } from '$lib/stores'
import type { ColumnDef } from '../apps/components/display/dbtable/utils'
import type { SelectedTable } from '../DBManager.svelte'
import { tableKeyOf } from '../dbRelations'

export type DiagramColumn = {
	name: string
	datatype: string
	isPrimaryKey: boolean
}

export type DiagramTable = {
	/** `schema.table`, the key relations and highlights are addressed by. */
	key: string
	schema: string
	table: string
	columns: DiagramColumn[]
}

/** Card geometry. The node component must render exactly the rows the layout
 * budgeted for, or cards overlap. */
export const CARD_WIDTH = 248
export const HEADER_HEIGHT = 34
export const ROW_HEIGHT = 22
const CARD_PADDING = 6
const GAP_X = 32
const GAP_Y = 20
/** Rows shown before a card collapses into a "+N more" row. */
export const COLLAPSED_ROW_LIMIT = 12
/** Cards are packed towards this width/height ratio, so the canvas opens on
 * something screen-shaped rather than one very long column. */
const TARGET_ASPECT = 1.7

export function visibleColumns(table: DiagramTable, expanded: boolean): DiagramColumn[] {
	return expanded ? table.columns : table.columns.slice(0, COLLAPSED_ROW_LIMIT)
}

export function hiddenColumnCount(table: DiagramTable, expanded: boolean): number {
	return table.columns.length - visibleColumns(table, expanded).length
}

/** Whether the card carries a row to expand or collapse its column list. */
export function hasExpandToggle(table: DiagramTable): boolean {
	return table.columns.length > COLLAPSED_ROW_LIMIT
}

export function cardHeight(table: DiagramTable, expanded: boolean): number {
	const rows = visibleColumns(table, expanded).length + (hasExpandToggle(table) ? 1 : 0)
	return HEADER_HEIGHT + rows * ROW_HEIGHT + CARD_PADDING
}

/** The tables to draw, in the order they are laid out. */
export function buildDiagramTables(
	dbSchema: DBSchema,
	colDefs: Record<string, ColumnDef[]> | undefined,
	selected: SelectedTable[]
): DiagramTable[] {
	if (dbSchema.lang === 'graphql') return []
	const tables: DiagramTable[] = []

	for (const { schema, table } of selected) {
		const key = tableKeyOf({ schema, table })
		const fromColDefs = colDefs?.[key]
		let columns: DiagramColumn[]
		if (fromColDefs?.length) {
			columns = fromColDefs.map((col) => ({
				name: col.field ?? '',
				datatype: col.datatype ?? '',
				isPrimaryKey: !!col.isprimarykey
			}))
		} else {
			// The cached schema has no primary-key flag; it only stands in until the
			// column metadata query lands.
			const schemaColumns = dbSchema.schema[schema]?.[table]
			if (!schemaColumns) continue
			columns = Object.entries(schemaColumns).map(([name, col]) => ({
				name,
				datatype: col.type,
				isPrimaryKey: false
			}))
		}
		tables.push({ key, schema, table, columns })
	}

	tables.sort((a, b) => a.schema.localeCompare(b.schema) || a.table.localeCompare(b.table))
	return tables
}

/** Packs the cards into balanced columns, in reading order. Runs again on every
 * change to the selection, so it must be deterministic: the same tables always
 * land in the same place. */
export function layoutTables(
	tables: DiagramTable[],
	expanded: Set<string>
): Record<string, { x: number; y: number }> {
	const heights = tables.map((t) => cardHeight(t, expanded.has(t.key)))
	const stacked = heights.reduce((a, b) => a + b, 0) + GAP_Y * Math.max(0, tables.length - 1)

	let columnCount = 1
	let bestScore = Infinity
	for (let c = 1; c <= tables.length; c++) {
		const width = c * CARD_WIDTH + (c - 1) * GAP_X
		const height = stacked / c
		// Compared in log space so twice-too-wide and twice-too-tall score alike.
		const score = Math.abs(Math.log(width / height / TARGET_ASPECT))
		if (score < bestScore) {
			bestScore = score
			columnCount = c
		}
	}

	const target = stacked / columnCount
	const positions: Record<string, { x: number; y: number }> = {}
	let column = 0
	let columnHeight = 0

	tables.forEach((table, i) => {
		const height = heights[i]
		// Overshooting the target by more than stopping short of it costs less in
		// the next column.
		const overshoot = columnHeight + height - target
		if (column < columnCount - 1 && columnHeight > 0 && overshoot > target - columnHeight) {
			column++
			columnHeight = 0
		}
		positions[table.key] = { x: column * (CARD_WIDTH + GAP_X), y: columnHeight }
		columnHeight += height + GAP_Y
	})

	return positions
}
