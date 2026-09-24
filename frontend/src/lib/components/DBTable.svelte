<script lang="ts" module>
	export type DbRowFilter = { column: string; value: unknown }
	export type DbForeignKeyTarget = { table: string; column: string; value: unknown }

	type Block =
		| { status: 'loading' }
		| { status: 'loaded'; rows: Record<string, unknown>[] }
		| { status: 'error' }

	const ROW_HEIGHT = 32
	const HEADER_HEIGHT = 30
	const BLOCK_SIZE = 100
	const MAX_BLOCKS = 20
	const OVERSCAN = 10
	const MIN_COL_WIDTH = 60
	const DEFAULT_COL_WIDTH = 150
	const MAX_AUTO_COL_WIDTH = 400
	// Header padding plus its sort and key icons.
	const HEADER_CHROME = 48
	const CELL_PADDING = 18
	// A double click starts with a click: following a foreign key waits this long
	// so the double click can edit the cell instead.
	const FK_CLICK_DELAY = 250

	function formatCell(value: unknown): string {
		if (value === null || value === undefined) return ''
		if (typeof value === 'object') return JSON.stringify(value)
		return String(value)
	}

	const hasForeignKeyValue = (value: unknown) =>
		value !== null && value !== undefined && value !== ''
</script>

<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { Button } from './common'
	import {
		ArrowDown,
		ArrowUp,
		ClipboardCopy,
		Download,
		KeyRound,
		Link,
		Pencil,
		Pin,
		PinOff,
		ArrowLeftToLine,
		ArrowRightToLine,
		Trash2
	} from 'lucide-svelte'
	import InsertRowDrawerButton from './apps/components/display/InsertRowDrawerButton.svelte'
	import type { IDbTableOps } from './dbOps'
	import type { TableEditorForeignKey } from './apps/components/display/dbtable/tableEditor'
	import type { ColumnDef } from './apps/components/display/dbtable/utils'
	import { untrack } from 'svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import FilterSearchbar from './FilterSearchbar.svelte'
	import {
		buildDbTableFilterSchema,
		dbColumnKind,
		exactColumnFilterValue,
		FREE_TEXT_KEY,
		unescapeFreeText,
		renderDbTableFilters
	} from './dbTableFilters'
	import ContextMenu, { type ContextMenuItem } from './common/contextmenu/ContextMenu.svelte'
	import GenericDropdown from './select/GenericDropdown.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { copyToClipboard, download } from '$lib/utils'
	import { readDbTableLayout, saveDbTableLayout, type DbTableLayout } from './dbTableLayoutStorage'
	import { convertJsonToCsv } from './table/tableUtils'
	import { twMerge } from 'tailwind-merge'
	import { deepEqual } from 'fast-equals'

	const operatingWorkspace = useOperatingWorkspace()

	type Props = {
		dbTableOps: IDbTableOps
		/** Foreign keys of the displayed table; their values link to the referenced row. */
		foreignKeys?: TableEditorForeignKey[]
		onGoToRow?: (target: DbForeignKeyTarget) => void
		/** Replaces the search bar's filters with `column = value` whenever a new
		 * one is passed, then `onRowFilterApplied` is called so it applies once. */
		rowFilter?: DbRowFilter
		onRowFilterApplied?: () => void
		/** localStorage key under which the columns the user resized or pinned are kept. */
		layoutStorageKey?: string
	}
	let {
		dbTableOps,
		foreignKeys,
		onGoToRow,
		rowFilter,
		onRowFilterApplied,
		layoutStorageKey
	}: Props = $props()

	let columns: ColumnDef[] = $derived(
		(dbTableOps.colDefs ?? []).filter((c) => c?.field && !c.ignored && !c.hide)
	)

	// ── Filters ──────────────────────────────────────────────────────────────
	let filterSchema = $derived(buildDbTableFilterSchema(columns))
	let filters: Record<string, any> = $state({})
	let quicksearch = $derived(unescapeFreeText(String(filters[FREE_TEXT_KEY] ?? '')))
	let filterWhere = $derived(
		renderDbTableFilters(filters, filterSchema, columns, dbTableOps.dbType)
	)
	let hasFilters = $derived(!!quicksearch || !!filterWhere)

	$effect(() => {
		const f = rowFilter
		if (!f) return
		untrack(() => {
			const key = filterSchema.keyOfColumn[f.column]
			if (key) {
				const datatype = columns.find((c) => c.field === f.column)?.datatype
				filters = { [key]: exactColumnFilterValue(datatype, f.value) }
			}
			onRowFilterApplied?.()
		})
	})

	// ── Foreign keys ─────────────────────────────────────────────────────────
	/** Single-column foreign keys only: a composite key has no one cell value to follow. */
	let fkByColumn = $derived.by(() => {
		const out: Record<string, { table: string; column: string }> = {}
		if (!onGoToRow) return out
		for (const fk of foreignKeys ?? []) {
			if (fk.columns.length !== 1) continue
			const { sourceColumn, targetColumn } = fk.columns[0]
			if (!sourceColumn || !targetColumn || !fk.targetTable) continue
			out[sourceColumn] = { table: fk.targetTable, column: targetColumn }
		}
		return out
	})

	let fkClickTimer: ReturnType<typeof setTimeout> | undefined
	function followForeignKey(column: string, value: unknown) {
		const fk = fkByColumn[column]
		if (!fk) return
		clearTimeout(fkClickTimer)
		const go = () => onGoToRow?.({ table: fk.table, column: fk.column, value })
		if (dbTableOps.onUpdate) fkClickTimer = setTimeout(go, FK_CLICK_DELAY)
		else go()
	}

	// ── Sorting ──────────────────────────────────────────────────────────────
	let sort: { column: string; desc: boolean } | undefined = $state()
	function toggleSort(column: string) {
		if (sort?.column !== column) sort = { column, desc: false }
		else if (!sort.desc) sort = { column, desc: true }
		else sort = undefined
	}

	// ── Rows (infinite scroll) ───────────────────────────────────────────────
	let blocks: Record<number, Block> = $state.raw({})
	/** Bumped on every reload so reads of an older query are dropped. */
	let generation = $state(0)
	/** Known once a block comes back short. */
	let endRow: number | undefined = $state()
	let rowCount: number | undefined = $state()
	let refreshCount = $state(0)
	export const refresh = () => (refreshCount += 1)

	let scrollEl: HTMLDivElement | undefined = $state()
	let scrollTop = $state(0)
	let viewportHeight = $state(0)
	let viewportWidth = $state(0)
	let gridOuterWidth = $state(0)
	let gridOuterHeight = $state(0)

	// Parents rebuild `dbTableOps` on unrelated updates: compare what it reads.
	let prevQuery: Record<string, unknown> | undefined
	$effect(() => {
		const query = {
			table: dbTableOps.tableKey,
			colDefs: dbTableOps.colDefs,
			quicksearch,
			filterWhere,
			sort,
			workspace: $operatingWorkspace,
			refreshCount
		}
		untrack(() => {
			if (deepEqual(query, prevQuery)) return
			// A refresh keeps the scroll position; a new query starts from the top.
			const onlyRefreshed =
				prevQuery && deepEqual({ ...query, refreshCount: 0 }, { ...prevQuery, refreshCount: 0 })
			prevQuery = query
			if (!onlyRefreshed && scrollEl) scrollEl.scrollTop = 0
			reload()
		})
	})

	function reload() {
		generation += 1
		blocks = {}
		endRow = undefined
		rowCount = undefined
		editing = undefined
		if (!$operatingWorkspace) return
		const gen = generation
		dbTableOps
			.getCount({ quicksearch, whereClause: filterWhere })
			.then((count) => gen === generation && (rowCount = count))
			.catch(() => {})
	}

	let totalRows = $derived.by(() => {
		if (endRow !== undefined) return endRow
		let loadedEnd = 0
		for (const [i, block] of Object.entries(blocks)) {
			if (block.status === 'loaded')
				loadedEnd = Math.max(loadedEnd, Number(i) * BLOCK_SIZE + block.rows.length)
		}
		// Unknown size: leave room for the next block so scrolling reaches it.
		return rowCount !== undefined ? Math.max(rowCount, loadedEnd) : loadedEnd + BLOCK_SIZE
	})

	let firstVisible = $derived(Math.floor(scrollTop / ROW_HEIGHT))
	let lastVisible = $derived(
		Math.min(totalRows, Math.ceil((scrollTop + viewportHeight - HEADER_HEIGHT) / ROW_HEIGHT))
	)
	let renderStart = $derived(Math.max(0, firstVisible - OVERSCAN))
	let renderEnd = $derived(Math.min(totalRows, lastVisible + OVERSCAN))
	let renderedIndices = $derived(
		Array.from({ length: Math.max(0, renderEnd - renderStart) }, (_, i) => renderStart + i)
	)

	$effect(() => {
		generation
		if (!$operatingWorkspace || renderEnd <= renderStart) return
		const from = Math.floor(renderStart / BLOCK_SIZE)
		const to = Math.floor((renderEnd - 1) / BLOCK_SIZE)
		untrack(() => {
			for (let b = from; b <= to; b++) if (!blocks[b]) loadBlock(b)
			evictBlocks((from + to) / 2)
		})
	})

	async function loadBlock(b: number) {
		const gen = generation
		blocks = { ...blocks, [b]: { status: 'loading' } }
		try {
			const rows = (await dbTableOps.getRows({
				offset: b * BLOCK_SIZE,
				limit: BLOCK_SIZE,
				quicksearch,
				whereClause: filterWhere,
				order_by: sort?.column ?? columns[0]?.field ?? '',
				is_desc: sort?.desc ?? false
			})) as Record<string, unknown>[]
			if (gen !== generation) return
			blocks = { ...blocks, [b]: { status: 'loaded', rows } }
			if (rows.length < BLOCK_SIZE) {
				const end = b * BLOCK_SIZE + rows.length
				endRow = endRow === undefined ? end : Math.min(endRow, end)
			}
		} catch (e) {
			if (gen !== generation) return
			blocks = { ...blocks, [b]: { status: 'error' } }
			sendUserToast(`Error loading rows: ${(e as Error)?.message ?? e}`, true)
		}
	}

	function evictBlocks(center: number) {
		const keys = Object.keys(blocks).map(Number)
		if (keys.length <= MAX_BLOCKS) return
		const keep = new Set(
			keys.sort((a, b) => Math.abs(a - center) - Math.abs(b - center)).slice(0, MAX_BLOCKS)
		)
		blocks = Object.fromEntries(Object.entries(blocks).filter(([k]) => keep.has(Number(k))))
	}

	function rowAt(i: number): Record<string, unknown> | undefined | 'loading' {
		const block = blocks[Math.floor(i / BLOCK_SIZE)]
		if (!block || block.status === 'loading') return 'loading'
		if (block.status === 'error') return undefined
		return block.rows[i % BLOCK_SIZE]
	}

	function loadedRows(): Record<string, unknown>[] {
		return Object.keys(blocks)
			.map(Number)
			.sort((a, b) => a - b)
			.flatMap((k) => {
				const block = blocks[k]
				return block.status === 'loaded' ? block.rows : []
			})
	}

	// ── Stored layout ────────────────────────────────────────────────────────
	// Widths are kept once settled, whether auto-sized from the content or resized by hand, so a
	// table reopens as it was left rather than re-fitting to whatever rows come first.
	function saveLayout() {
		if (!layoutStorageKey) return
		const layout: DbTableLayout = {}
		const sizedWidths = Object.fromEntries(
			[...sizedColumns].filter((f) => widths[f] !== undefined).map((f) => [f, widths[f]])
		)
		if (Object.keys(sizedWidths).length) layout.widths = sizedWidths
		if (Object.keys(pinned).length) layout.pinned = $state.snapshot(pinned)
		saveDbTableLayout(layoutStorageKey, layout)
	}
	const storedLayout = untrack(() =>
		layoutStorageKey ? readDbTableLayout(layoutStorageKey) : ({} as DbTableLayout)
	)

	// ── Column widths ────────────────────────────────────────────────────────
	let widths: Record<string, number> = $state({ ...storedLayout.widths })
	let sizedColumns = new Set<string>(Object.keys(storedLayout.widths ?? {}))
	let autoSizedFor: ColumnDef[] | undefined

	let measureCtx: CanvasRenderingContext2D | null | undefined
	function textWidth(text: string, font: string): number {
		measureCtx ??= document.createElement('canvas').getContext('2d')
		if (!measureCtx) return text.length * 7
		measureCtx.font = font
		return measureCtx.measureText(text).width
	}

	function autoSize(only?: string[]) {
		if (!scrollEl) return
		const family = getComputedStyle(scrollEl).fontFamily
		const rows = loadedRows().slice(0, 300)
		const next = { ...widths }
		for (const col of columns) {
			if (only ? !only.includes(col.field) : sizedColumns.has(col.field)) continue
			let w = textWidth(col.field, `600 12px ${family}`) + HEADER_CHROME
			for (const row of rows) {
				const text = formatCell(row[col.field]).slice(0, 200) || 'NULL'
				w = Math.max(w, textWidth(text, `400 12px ${family}`) + CELL_PADDING)
			}
			next[col.field] = Math.round(Math.min(MAX_AUTO_COL_WIDTH, Math.max(MIN_COL_WIDTH, w)))
			sizedColumns.add(col.field)
		}
		widths = next
		saveLayout()
	}

	// Sized once per column set, from the first rows: later blocks would make
	// columns jump while scrolling.
	$effect(() => {
		const first = blocks[0]
		if (!first || first.status === 'loading' || autoSizedFor === columns) return
		untrack(() => {
			autoSizedFor = columns
			autoSize()
		})
	})

	let colWidth = (field: string) => widths[field] ?? DEFAULT_COL_WIDTH
	let totalWidth = $derived(columns.reduce((acc, c) => acc + colWidth(c.field), 0))

	// ── Pinning ──────────────────────────────────────────────────────────────
	let pinned: Record<string, 'left' | 'right'> = $state({ ...storedLayout.pinned })
	let displayColumns = $derived([
		...columns.filter((c) => pinned[c.field] === 'left'),
		...columns.filter((c) => !pinned[c.field]),
		...columns.filter((c) => pinned[c.field] === 'right')
	])
	let firstRightPinned = $derived(displayColumns.find((c) => pinned[c.field] === 'right')?.field)
	let lastLeftPinned = $derived(displayColumns.findLast((c) => pinned[c.field] === 'left')?.field)
	/** Sticky offset of each pinned column, from the edge it is pinned to. */
	let pinOffsets = $derived.by(() => {
		const out: Record<string, number> = {}
		let acc = 0
		for (const c of displayColumns) {
			if (pinned[c.field] !== 'left') continue
			out[c.field] = acc
			acc += colWidth(c.field)
		}
		acc = 0
		for (const c of [...displayColumns].reverse()) {
			if (pinned[c.field] !== 'right') continue
			out[c.field] = acc
			acc += colWidth(c.field)
		}
		return out
	})
	let pinnedWidth = (side: 'left' | 'right') =>
		columns.reduce((acc, c) => (pinned[c.field] === side ? acc + colWidth(c.field) : acc), 0)

	function setPin(column: string, side: 'left' | 'right' | undefined) {
		const next = { ...pinned }
		if (side) next[column] = side
		else delete next[column]
		pinned = next
		saveLayout()
	}

	function startResize(e: PointerEvent, field: string) {
		e.preventDefault()
		e.stopPropagation()
		const handle = e.currentTarget as HTMLElement
		handle.setPointerCapture(e.pointerId)
		const startX = e.clientX
		const startWidth = colWidth(field)
		const onMove = (ev: PointerEvent) => {
			sizedColumns.add(field)
			widths = {
				...widths,
				[field]: Math.max(MIN_COL_WIDTH, Math.round(startWidth + ev.clientX - startX))
			}
		}
		const onUp = () => {
			handle.removeEventListener('pointermove', onMove)
			handle.removeEventListener('pointerup', onUp)
			if (colWidth(field) !== startWidth) saveLayout()
		}
		handle.addEventListener('pointermove', onMove)
		handle.addEventListener('pointerup', onUp)
	}

	// ── Selection & keyboard ─────────────────────────────────────────────────
	let selected: { row: number; column: string } | undefined = $state()

	function cellFromEvent(e: Event): { row: number; column: string } | undefined {
		const el = (e.target as HTMLElement | null)?.closest<HTMLElement>('[data-cell-row]')
		if (!el) return undefined
		const cell = { row: Number(el.dataset.cellRow), column: el.dataset.cellColumn! }
		const data = rowAt(cell.row)
		return data && data !== 'loading' ? cell : undefined
	}

	function headerFromEvent(e: Event): string | undefined {
		return (e.target as HTMLElement | null)?.closest<HTMLElement>('[data-header-column]')?.dataset
			.headerColumn
	}

	function scrollIntoView(cell: { row: number; column: string }) {
		if (!scrollEl) return
		const top = cell.row * ROW_HEIGHT
		const bodyHeight = viewportHeight - HEADER_HEIGHT
		if (top < scrollEl.scrollTop) scrollEl.scrollTop = top
		else if (top + ROW_HEIGHT > scrollEl.scrollTop + bodyHeight)
			scrollEl.scrollTop = top + ROW_HEIGHT - bodyHeight
		// A pinned column is always in view; the others must clear the pinned ones.
		if (pinned[cell.column]) return
		let left = 0
		for (const c of displayColumns) {
			if (c.field === cell.column) break
			left += colWidth(c.field)
		}
		const width = colWidth(cell.column)
		const visibleLeft = scrollEl.scrollLeft + pinnedWidth('left')
		const visibleRight = scrollEl.scrollLeft + viewportWidth - pinnedWidth('right')
		if (left < visibleLeft) scrollEl.scrollLeft = left - pinnedWidth('left')
		else if (left + width > visibleRight)
			scrollEl.scrollLeft = left + width - viewportWidth + pinnedWidth('right')
	}

	function onGridKeyDown(e: KeyboardEvent) {
		if (!selected || editing) return
		const colIdx = displayColumns.findIndex((c) => c.field === selected!.column)
		let next: { row: number; column: string } | undefined
		switch (e.key) {
			case 'ArrowDown':
				next = { ...selected, row: Math.min(totalRows - 1, selected.row + 1) }
				break
			case 'ArrowUp':
				next = { ...selected, row: Math.max(0, selected.row - 1) }
				break
			case 'ArrowRight':
				next = {
					...selected,
					column: displayColumns[Math.min(displayColumns.length - 1, colIdx + 1)].field
				}
				break
			case 'ArrowLeft':
				next = { ...selected, column: displayColumns[Math.max(0, colIdx - 1)].field }
				break
			case 'Enter':
			case 'F2':
				startEdit(selected.row, selected.column)
				break
			case 'c':
				if (!(e.ctrlKey || e.metaKey) || window.getSelection()?.toString()) return
				copyCell(selected.row, selected.column)
				break
			default:
				return
		}
		e.preventDefault()
		if (next) {
			selected = next
			scrollIntoView(next)
		}
	}

	function copyCell(row: number, column: string) {
		const data = rowAt(row)
		if (!data || data === 'loading') return
		copyToClipboard(formatCell(data[column]))
	}

	// ── Editing ──────────────────────────────────────────────────────────────
	let editing:
		| { row: number; column: string; data: Record<string, unknown>; initial: string; value: string }
		| undefined = $state()
	let editorInput: TextInput<'input'> | undefined = $state()
	let editorRect = new DOMRect()

	function cellElement(row: number, column: string): HTMLElement | null | undefined {
		return scrollEl?.querySelector<HTMLElement>(
			`[data-cell-row="${row}"][data-cell-column="${CSS.escape(column)}"]`
		)
	}

	function startEdit(row: number, column: string) {
		clearTimeout(fkClickTimer)
		if (!dbTableOps.onUpdate) return
		const data = rowAt(row)
		if (!data || data === 'loading') return
		selected = { row, column }
		const initial = formatCell(data[column])
		editing = { row, column, data, initial, value: initial }
		requestAnimationFrame(() => {
			editorInput?.focus()
			editorInput?.select()
		})
	}

	async function commitEdit() {
		const edit = editing
		editing = undefined
		scrollEl?.focus({ preventScroll: true })
		if (!edit || edit.value === edit.initial || !$operatingWorkspace || !dbTableOps.onUpdate) return
		const colDef = columns.find((c) => c.field === edit.column)
		if (!colDef) return
		// Shown right away; a failed update reloads the real value.
		const b = Math.floor(edit.row / BLOCK_SIZE)
		const block = blocks[b]
		if (block?.status === 'loaded') {
			const rows = [...block.rows]
			rows[edit.row % BLOCK_SIZE] = { ...edit.data, [edit.column]: edit.value }
			blocks = { ...blocks, [b]: { status: 'loaded', rows } }
		}
		try {
			await dbTableOps.onUpdate({ values: edit.data }, colDef, edit.value)
			sendUserToast('Value updated')
		} catch (e) {
			sendUserToast('Error updating value: ' + ((e as Error)?.message || e), true)
			refresh()
		}
	}

	function cancelEdit() {
		editing = undefined
		scrollEl?.focus({ preventScroll: true })
	}

	// ── Context menu ─────────────────────────────────────────────────────────
	let menuTarget:
		| { kind: 'cell'; row: number; column: string }
		| { kind: 'header'; column: string }
		| undefined = $state()

	// Runs in the capture phase, before the menu's own handler opens it, so the items are
	// already those of what was clicked. Anywhere else, the browser keeps its own menu.
	// A right-click while the menu is open would leave it where it first opened: close it,
	// hold the click until the old menu is gone, then replay it.
	let replayingContextMenu = false
	function replayOnceMenuClosed(e: MouseEvent) {
		const target = e.target as HTMLElement
		const init: MouseEventInit = {
			bubbles: true,
			cancelable: true,
			clientX: e.clientX,
			clientY: e.clientY,
			button: 2
		}
		document
			.querySelector('[data-context-menu]')
			?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
		const started = performance.now()
		const replay = () => {
			if (document.querySelector('[data-context-menu]') && performance.now() - started < 500) {
				requestAnimationFrame(replay)
				return
			}
			replayingContextMenu = true
			target.dispatchEvent(new MouseEvent('contextmenu', init))
			replayingContextMenu = false
		}
		requestAnimationFrame(replay)
	}

	function onContextMenuCapture(e: MouseEvent) {
		if (!replayingContextMenu && document.querySelector('[data-context-menu]')) {
			e.preventDefault()
			e.stopPropagation()
			replayOnceMenuClosed(e)
			return
		}
		const cell = cellFromEvent(e)
		if (cell) {
			menuTarget = { kind: 'cell', ...cell }
			selected = cell
			return
		}
		selected = undefined
		const column = headerFromEvent(e)
		if (column) {
			menuTarget = { kind: 'header', column }
			return
		}
		menuTarget = undefined
		e.stopPropagation()
	}

	let menuItems: ContextMenuItem[] = $derived.by(() => {
		if (!menuTarget) return []
		if (menuTarget.kind === 'header') {
			const { column } = menuTarget
			const pin = pinned[column]
			const items: ContextMenuItem[] = []
			if (pin !== 'left')
				items.push({
					id: 'pin-left',
					label: 'Pin to left',
					icon: ArrowLeftToLine,
					onClick: () => setPin(column, 'left')
				})
			if (pin !== 'right')
				items.push({
					id: 'pin-right',
					label: 'Pin to right',
					icon: ArrowRightToLine,
					onClick: () => setPin(column, 'right')
				})
			if (pin)
				items.push({
					id: 'unpin',
					label: 'Unpin',
					icon: PinOff,
					onClick: () => setPin(column, undefined)
				})
			return items
		}
		const { row, column } = menuTarget
		const items: ContextMenuItem[] = [
			{
				id: 'copy-value',
				label: 'Copy value',
				icon: ClipboardCopy,
				onClick: () => copyCell(row, column)
			},
			{
				id: 'copy-row',
				label: 'Copy row as JSON',
				icon: ClipboardCopy,
				onClick: () => {
					const data = rowAt(row)
					if (data && data !== 'loading') copyToClipboard(JSON.stringify(data, null, 2))
				}
			}
		]
		if (dbTableOps.onUpdate) {
			items.push({
				id: 'edit',
				label: 'Edit value',
				icon: Pencil,
				onClick: () => startEdit(row, column)
			})
		}
		if (dbTableOps.onDelete) {
			items.push(
				{ id: 'divider', label: '', divider: true },
				{
					id: 'delete',
					label: 'Delete row',
					icon: Trash2,
					type: 'delete',
					onClick: () => deleteRow(row)
				}
			)
		}
		return items
	})

	function deleteRow(row: number) {
		const data = rowAt(row)
		if (!data || data === 'loading' || !$operatingWorkspace) return
		dbTableOps
			.onDelete?.({ values: data })
			.then(() => {
				refresh()
				sendUserToast('Row deleted')
			})
			.catch((e) => sendUserToast(`Error deleting row: ${e?.message ?? e}`, true))
	}

	function downloadCsv() {
		const rows = loadedRows()
		if (!rows.length) return
		download(`${dbTableOps.tableKey}.csv`, convertJsonToCsv(rows), 'text/csv')
	}
</script>

<svelte:window onmousedown={() => editing && cancelEdit()} />

<div class="h-full relative flex flex-col">
	<div class="flex justify-between items-center bg-surface-input pr-2">
		<FilterSearchbar
			class="grow rounded-none !border-0"
			size="lg"
			searchIconPosition="left"
			dropdownMaxWidth={480}
			dropdownOffsetY={1}
			schema={filterSchema.schema}
			bind:value={filters}
			placeholder="Search, or filter by column..."
		/>
		<div class="flex shrink-0 items-center gap-2 text-xs text-secondary">
			{#if totalRows > 0 && endRow !== 0}
				<span class="whitespace-nowrap">
					{Math.min(firstVisible + 1, totalRows)}–{Math.min(lastVisible, totalRows)}
					{#if rowCount !== undefined || endRow !== undefined}
						of {endRow ?? rowCount} rows
					{/if}
				</span>
			{/if}
			<Button
				startIcon={{ icon: Download }}
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				title="Download loaded rows as CSV"
				onClick={downloadCsv}
			/>
			{#if dbTableOps.onInsert}
				<InsertRowDrawerButton
					columnDefs={dbTableOps.colDefs ?? []}
					dbType={dbTableOps.dbType}
					onInsert={(values) => {
						if (!$operatingWorkspace) return
						dbTableOps.onInsert?.({ values }).then(() => {
							refresh()
							sendUserToast('Row inserted')
						})
					}}
				/>
			{/if}
		</div>
	</div>
	<div
		class="relative flex flex-col flex-1 min-h-0 overflow-hidden bg-surface-tertiary border-t"
		oncontextmenucapture={onContextMenuCapture}
	>
		<ContextMenu items={menuItems} class="flex-1 min-h-0 cursor-auto" zIndex="z-[9999]">
			<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
			<div
				bind:this={scrollEl}
				bind:clientHeight={viewportHeight}
				bind:clientWidth={viewportWidth}
				bind:offsetHeight={gridOuterHeight}
				bind:offsetWidth={gridOuterWidth}
				class={twMerge(
					'h-full w-full overflow-x-auto overflow-y-scroll relative outline-none text-xs text-primary',
					// Wider than the app's default so it is easy to grab.
					'[&::-webkit-scrollbar]:w-2.5 [&::-webkit-scrollbar]:h-2.5 [&::-webkit-scrollbar-thumb]:rounded-full'
				)}
				tabindex="0"
				role="grid"
				aria-rowcount={totalRows}
				data-testid="db-table-grid"
				onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
				onkeydown={onGridKeyDown}
				onclick={(e) => (selected = cellFromEvent(e))}
				ondblclick={(e) => {
					const cell = cellFromEvent(e)
					if (cell) startEdit(cell.row, cell.column)
				}}
			>
				<div
					class="relative min-w-full"
					style:width="{totalWidth}px"
					style:height="{HEADER_HEIGHT + totalRows * ROW_HEIGHT}px"
				>
					<!-- Clipped because resize handles straddle the cell edges: a column pinned to the
						 right would push its handle past the grid and scroll it sideways. -->
					<div
						class="sticky top-0 z-10 flex overflow-x-clip bg-surface-tertiary border-b"
						style:height="{HEADER_HEIGHT}px"
						role="row"
					>
						{#each displayColumns as col (col.field)}
							{@const sorted = sort?.column === col.field ? sort : undefined}
							{@const pin = pinned[col.field]}
							{#if col.field === firstRightPinned}
								<div class="flex-1"></div>
							{/if}
							<div
								class={twMerge(
									'relative shrink-0 flex items-center border-r',
									// Above the other header cells, whose resize handles sit at z-10.
									pin ? 'sticky z-20 bg-surface-tertiary' : '',
									// The pinned block's edge against the scrolling columns is a pixel thicker.
									col.field === lastLeftPinned ? 'border-r-2' : '',
									col.field === firstRightPinned ? 'border-l-2' : ''
								)}
								style:width="{colWidth(col.field)}px"
								style:left={pin === 'left' ? `${pinOffsets[col.field]}px` : undefined}
								style:right={pin === 'right' ? `${pinOffsets[col.field]}px` : undefined}
								role="columnheader"
								aria-sort={sorted ? (sorted.desc ? 'descending' : 'ascending') : 'none'}
								data-header-column={col.field}
							>
								<button
									class="flex items-center gap-1 w-full h-full px-2 font-semibold text-emphasis text-left hover:bg-surface-hover min-w-0"
									title="{col.field} ({col.datatype})"
									onclick={() => toggleSort(col.field)}
								>
									{#if col.isprimarykey}
										<KeyRound size={12} class="shrink-0 text-secondary" />
									{:else if fkByColumn[col.field]}
										<Link size={12} class="shrink-0 text-secondary" />
									{/if}
									<span class="truncate">{col.field}</span>
									{#if pin}
										<Pin size={11} class="shrink-0 text-hint" />
									{/if}
									{#if sorted}
										{@const Icon = sorted.desc ? ArrowDown : ArrowUp}
										<Icon size={12} class="shrink-0 text-secondary" />
									{/if}
								</button>
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class="absolute -right-1 top-0 bottom-0 w-2 z-10 cursor-col-resize hover:bg-border-selected/50"
									title="Drag to resize, double click to fit"
									onpointerdown={(e) => startResize(e, col.field)}
									ondblclick={(e) => {
										e.stopPropagation()
										autoSize([col.field])
									}}
								></div>
							</div>
						{/each}
						{#if !firstRightPinned}
							<div class="flex-1"></div>
						{/if}
					</div>

					{#if endRow === 0}
						<div class="sticky left-0 py-8 text-center text-hint" style:width="{viewportWidth}px">
							{hasFilters ? 'No rows match the filters' : 'No rows'}
						</div>
					{/if}

					<div
						class="absolute left-0 right-0"
						style:top="{HEADER_HEIGHT + renderStart * ROW_HEIGHT}px"
					>
						{#each renderedIndices as i (i)}
							{@const data = rowAt(i)}
							<div
								class="group flex border-b hover:bg-surface-hover"
								style:height="{ROW_HEIGHT}px"
								role="row"
								aria-rowindex={i + 1}
							>
								{#each displayColumns as col (col.field)}
									{@const isSelected = selected?.row === i && selected.column === col.field}
									{@const pin = pinned[col.field]}
									{#if col.field === firstRightPinned}
										<div class="flex-1"></div>
									{/if}
									<div
										class={twMerge(
											'shrink-0 px-2 border-r truncate select-none',
											dbColumnKind(col.datatype) === 'number' ? 'text-right tabular-nums' : '',
											pin ? 'sticky z-[1] bg-surface-tertiary' : '',
											col.field === lastLeftPinned ? 'border-r-2' : '',
											col.field === firstRightPinned ? 'border-l-2' : '',
											isSelected ? 'outline outline-1 -outline-offset-1 outline-border-accent' : ''
										)}
										style:width="{colWidth(col.field)}px"
										style:line-height="{ROW_HEIGHT - 1}px"
										style:left={pin === 'left' ? `${pinOffsets[col.field]}px` : undefined}
										style:right={pin === 'right' ? `${pinOffsets[col.field]}px` : undefined}
										role="gridcell"
										data-cell-row={i}
										data-cell-column={col.field}
									>
										{#if pin}
											<!-- The opaque background hides the row's hover tint; this restores it. -->
											<div class="pointer-events-none absolute inset-0 group-hover:bg-surface-hover"
											></div>
										{/if}
										{#if data === 'loading'}
											<div class="inline-block h-2 w-2/3 rounded bg-surface-secondary animate-pulse"
											></div>
										{:else if data}
											{@const value = data[col.field]}
											{#if value === null || value === undefined}
												<span class="italic text-hint">NULL</span>
											{:else if fkByColumn[col.field] && hasForeignKeyValue(value)}
												{@const fk = fkByColumn[col.field]}
												<button
													class="text-accent hover:underline underline-offset-2 max-w-full truncate"
													title="Go to {fk.table} where {fk.column} = {formatCell(value)}"
													data-testid="db-fk-link"
													onclick={() => followForeignKey(col.field, value)}
												>
													{formatCell(value)}
												</button>
											{:else}
												{formatCell(value)}
											{/if}
										{/if}
									</div>
								{/each}
								{#if !firstRightPinned}
									<div class="flex-1"></div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			</div>
		</ContextMenu>
		<!-- Lines between the rows and the scrollbars, drawn over the grid: scrollbar styling
			 cannot carry a border in every browser. None with overlay scrollbars, which take no room. -->
		{#if gridOuterWidth - viewportWidth > 0}
			<div
				class="pointer-events-none absolute top-0 z-20 w-px bg-border-light"
				style:right="{gridOuterWidth - viewportWidth}px"
				style:height="{viewportHeight}px"
			></div>
		{/if}
		{#if gridOuterHeight - viewportHeight > 0}
			<div
				class="pointer-events-none absolute left-0 z-20 h-px bg-border-light"
				style:top="{viewportHeight}px"
				style:width="{viewportWidth}px"
			></div>
		{/if}
	</div>
</div>

<GenericDropdown
	open={!!editing}
	getInputRect={() => {
		if (editing) {
			const el = cellElement(editing.row, editing.column)
			if (el) editorRect = el.getBoundingClientRect()
		}
		return editorRect
	}}
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="p-2 flex flex-col gap-1 min-w-72" onmousedown={(e) => e.stopPropagation()}>
		{#if editing}
			<TextInput
				bind:this={editorInput}
				bind:value={editing.value}
				size="sm"
				inputProps={{
					onkeydown: (e) => {
						if (e.key === 'Enter') {
							e.preventDefault()
							commitEdit()
						} else if (e.key === 'Escape') {
							e.preventDefault()
							e.stopPropagation()
							cancelEdit()
						}
					}
				}}
			/>
			<div class="text-2xs text-hint">Enter to save, Esc to cancel</div>
		{/if}
	</div>
</GenericDropdown>
