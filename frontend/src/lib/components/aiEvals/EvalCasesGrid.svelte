<script lang="ts">
	import { createGrid, type GridApi } from 'ag-grid-community'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import '$lib/components/apps/components/display/table/theme/windmill-theme.css'
	import { transformColumnDefs } from '$lib/components/apps/components/display/table/utils'
	import { multilineCellColDef } from '$lib/components/apps/components/display/table/multilineCellEditor'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { untrack } from 'svelte'
	import type { CaseDraft } from './evalUtils'

	let {
		cases = $bindable(),
		onRemove,
		locked = false,
		onEditingChange
	}: {
		/** The drawer's working copy. Edits land here as they are made; the drawer writes them. */
		cases: CaseDraft[]
		/** Asked before a row goes, since a stored case has runs that executed it. */
		onRemove: (c: CaseDraft) => void
		/** The cases are being written: an edit made now is one the request already left behind. */
		locked?: boolean
		/** A cell opened or closed. Reported up so Save can be pressed for an edit the list does not
		 *  hold yet: the press is what commits the cell, so it has to reach a live button. */
		onEditingChange?: (editing: boolean) => void
	} = $props()

	type Row = { id: string; question: string; expected: string }

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
		...multilineCellColDef
	}

	$effect(() => eGui && untrack(() => mountGrid()))
	function mountGrid() {
		if (!eGui || api) return
		createGrid(eGui, {
			rowData: untrack(() => cases.map(toRow)),
			columnDefs: columnDefs(),
			defaultColDef: { ...defaultColDef, editable: untrack(() => !locked) },
			onCellValueChanged: (e) => {
				const target = cases.find((c) => c.id === (e.data as Row).id)
				if (!target) return
				if (e.colDef.field === 'question') {
					target.input = { ...target.input, user_message: e.newValue ?? '' }
				} else if (e.colDef.field === 'expected') {
					setExpected(target, e.newValue ?? '')
				}
			},
			onCellEditingStarted: () => onEditingChange?.(true),
			onCellEditingStopped: () => onEditingChange?.(false),
			suppressColumnMoveAnimation: true,
			suppressDragLeaveHidesColumns: true,
			onGridReady: (e) => (api = e.api)
		})
	}

	/** Text that parses as JSON is stored as JSON, so a structured answer can be written by hand. */
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
				if (locked) return
				const target = cases.find((c) => c.id === (values as Row).id)
				if (target) onRemove(target)
			}
		})
	}

	// Keyed on which cases are in the list rather than on what is in them: pushing rows back on
	// each keystroke would reset the cell being typed in.
	let rowKey = $derived(cases.map((c) => c.id).join(','))
	$effect(() => {
		rowKey
		untrack(() => api?.updateGridOptions({ rowData: cases.map(toRow) }))
	})

	$effect(() => {
		const editable = !locked
		untrack(() => api?.updateGridOptions({ defaultColDef: { ...defaultColDef, editable } }))
	})

	/** Commit whatever cell is open into `cases`, and wait for it to land there: the drawer reads
	 *  the list to save it, and a cell still being typed in is an edit that press is saving. */
	export async function flush() {
		api?.stopEditing()
		// The commit reaches `cases` through the grid's own event queue, a macrotask away: reading in
		// the same turn, or after a microtask, reads the list as it was before the cell was typed in.
		await new Promise((resolve) => setTimeout(resolve, 0))
	}
</script>

<DarkModeObserver bind:darkMode />

<div bind:this={eGui} class="h-full w-full ag-theme-alpine {darkMode ? 'ag-theme-alpine-dark' : ''}"
></div>
