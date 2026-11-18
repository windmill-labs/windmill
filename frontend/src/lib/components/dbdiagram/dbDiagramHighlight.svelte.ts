import { getContext, setContext } from 'svelte'
import { columnKeyIn, type RelationIndex } from '../dbRelations'

/** What the pointer is on: a whole table, or one of its columns. */
export type HighlightTarget = { table: string; column?: string }

const EMPTY: ReadonlySet<string> = new Set()

/**
 * The diagram draws no edges, so the relationships only exist while something is
 * highlighted. This owns that: what is hovered (or pinned by a click), and which
 * tables and columns it lights up.
 */
export class DbDiagramHighlight {
	#getIndex: () => RelationIndex

	hovered = $state<HighlightTarget | undefined>(undefined)
	/** Set by clicking, so the relations can be read without holding the pointer
	 * still. A hover takes precedence while it lasts. */
	pinned = $state<HighlightTarget | undefined>(undefined)

	target = $derived(this.hovered ?? this.pinned)

	/** Tables that stay lit. Undefined means nothing is highlighted and every
	 * table renders at full strength. */
	tables = $derived.by<ReadonlySet<string> | undefined>(() => {
		const target = this.target
		if (!target) return undefined
		if (target.column) {
			const columnTable = this.#getIndex().columnTable
			const tables = new Set([target.table])
			for (const column of this.columns ?? EMPTY) {
				const table = columnTable.get(column)
				if (table) tables.add(table)
			}
			return tables
		}
		return new Set([target.table, ...(this.#getIndex().relatedTables.get(target.table) ?? EMPTY)])
	})

	/** Columns that stay lit inside the highlighted tables. */
	columns = $derived.by<ReadonlySet<string> | undefined>(() => {
		const target = this.target
		if (!target) return undefined
		const index = this.#getIndex()
		if (target.column) {
			const key = columnKeyIn(target.table, target.column)
			return new Set([key, ...(index.linkedColumns.get(key) ?? EMPTY)])
		}
		return index.relatedColumns.get(target.table) ?? EMPTY
	})

	constructor(getIndex: () => RelationIndex) {
		this.#getIndex = getIndex
	}

	hover(target: HighlightTarget | undefined) {
		this.hovered = target
	}

	/** Clicking the already-pinned target unpins it. */
	togglePin(target: HighlightTarget) {
		const current = this.pinned
		this.pinned =
			current && current.table === target.table && current.column === target.column
				? undefined
				: target
	}

	clear() {
		this.hovered = undefined
		this.pinned = undefined
	}
}

const contextKey = 'DbDiagramHighlight'

export const setDbDiagramHighlight = (highlight: DbDiagramHighlight) =>
	setContext(contextKey, highlight)
export const getDbDiagramHighlight = () => getContext<DbDiagramHighlight>(contextKey)
