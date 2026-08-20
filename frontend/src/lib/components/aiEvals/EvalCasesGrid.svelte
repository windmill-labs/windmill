<script lang="ts">
	import { createGrid, type GridApi } from 'ag-grid-community'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import '$lib/components/apps/components/display/table/theme/windmill-theme.css'
	import { transformColumnDefs } from '$lib/components/apps/components/display/table/utils'
	import { multilineCellColDef } from '$lib/components/apps/components/display/table/multilineCellEditor'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { untrack } from 'svelte'
	import type { CaseDraft } from './evalCaseUtils'

	let {
		cases = $bindable(),
		onRemove,
		locked = false,
		editing = $bindable(false)
	}: {
		/** The drawer's working copy. Edits land here as they are made; the drawer writes them. */
		cases: CaseDraft[]
		/** Asked before a row goes, since a stored case has runs that executed it. */
		onRemove: (c: CaseDraft) => void
		/** The cases are being written. An edit made now would be one the request already left
		 *  behind, and the drawer closes on the response, so it would go without being saved. */
		locked?: boolean
		/** A cell is open, holding an edit the list does not have yet. Reported up so Save can be
		 *  pressed for it: the press is what commits the cell, so it has to reach a live button. */
		editing?: boolean
	} = $props()

	type Row = { id: string; question: string; expected: string }

	/** A stored answer is text or JSON; a cell is text either way, and what it parses to is what
	 *  the case carries. Shown as formatted JSON so a structured answer is readable. */
	function expectedToText(value: unknown): string {
		if (value == undefined) return ''
		return typeof value === 'string' ? value : JSON.stringify(value, null, 2)
	}

	function toRow(c: CaseDraft): Row {
		return {
			id: c.id ?? '',
			question: c.input?.user_message ?? '',
			expected: expectedToText(c.expected)
		}
	}

	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()
	let darkMode = $state(false)

	const defaultColDef = {
		flex: 1,
		minWidth: 120,
		// A case is prose, so a cell has to be able to hold a newline. The editor stays the height
		// of the row until one is added, which is what the rest of this app's grids look like while
		// you type in them.
		...multilineCellColDef
	}

	$effect(() => eGui && untrack(() => mountGrid()))
	function mountGrid() {
		if (!eGui || api) return
		createGrid(eGui, {
			rowData: untrack(() => cases.map(toRow)),
			columnDefs: columnDefs(),
			defaultColDef: { ...defaultColDef, editable: untrack(() => !locked) },
			// The rows are held here rather than fetched, so the grid virtualises them: a dataset is
			// capped at a thousand cases, and a row per case in the DOM is what makes a table that
			// size stop responding.
			onCellValueChanged: (e) => {
				const target = cases.find((c) => c.id === (e.data as Row).id)
				if (!target) return
				if (e.colDef.field === 'question') {
					target.input = { ...target.input, user_message: e.newValue ?? '' }
				} else if (e.colDef.field === 'expected') {
					setExpected(target, e.newValue ?? '')
				}
			},
			onCellEditingStarted: () => (editing = true),
			onCellEditingStopped: () => (editing = false),
			suppressColumnMoveAnimation: true,
			suppressDragLeaveHidesColumns: true,
			onGridReady: (e) => (api = e.api)
		})
	}

	/** Text that parses as JSON is stored as JSON, so a structured answer can be written by hand
	 *  rather than only produced by a run. */
	function setExpected(c: CaseDraft, text: string) {
		const trimmed = text.trim()
		if (!trimmed) {
			c.expected = undefined
			return
		}
		try {
			c.expected = JSON.parse(text)
		} catch {
			c.expected = text
		}
	}

	function columnDefs() {
		return transformColumnDefs({
			columnDefs: [
				{ field: 'question', headerName: 'Question', flex: 3 },
				{ field: 'expected', headerName: 'Expected', flex: 2 }
			] as any,
			onDelete: (values) => {
				// Locked for the same reason the cells are: the list is being written, and a row
				// dropped now is one the request already left behind.
				if (locked) return
				const target = cases.find((c) => c.id === (values as Row).id)
				if (target) onRemove(target)
			}
		})
	}

	// Keyed on which cases are in the list rather than on what is in them: the grid already holds
	// every edit that came out of it, and pushing rows back on each keystroke would reset the cell
	// being typed in.
	let rowKey = $derived(cases.map((c) => c.id).join(','))
	$effect(() => {
		rowKey
		untrack(() => api?.updateGridOptions({ rowData: cases.map(toRow) }))
	})

	$effect(() => {
		const editable = !locked
		untrack(() => api?.updateGridOptions({ defaultColDef: { ...defaultColDef, editable } }))
	})

	/**
	 * Commit whatever cell is open into `cases`, synchronously.
	 *
	 * Called by the drawer before it reads the list to save it. `stopEditing` fires
	 * `onCellValueChanged` in the same tick, so the value is in `cases` by the time this returns —
	 * which is the whole point: a cell someone is still typing in when they press Save is an edit
	 * they made, and an effect reacting to the save would run a microtask too late to catch it.
	 */
	export async function flush() {
		api?.stopEditing()
		// The commit reaches `cases` through the grid's own event queue, which is a macrotask away:
		// reading the list in the same turn — or after a microtask — reads it as it was before the
		// cell was typed in, which is exactly the edit being saved.
		await new Promise((resolve) => setTimeout(resolve, 0))
	}
</script>

<DarkModeObserver bind:darkMode />

<div bind:this={eGui} class="h-full w-full ag-theme-alpine {darkMode ? 'ag-theme-alpine-dark' : ''}"
></div>
