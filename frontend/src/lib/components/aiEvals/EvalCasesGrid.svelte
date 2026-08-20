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
		onRemove
	}: {
		/** The drawer's working copy. Edits land here as they are made; the drawer writes them. */
		cases: CaseDraft[]
		/** Asked before a row goes, since a stored case has runs that executed it. */
		onRemove: (c: CaseDraft) => void
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

	$effect(() => eGui && untrack(() => mountGrid()))
	function mountGrid() {
		if (!eGui || api) return
		createGrid(eGui, {
			rowData: untrack(() => cases.map(toRow)),
			columnDefs: columnDefs(),
			defaultColDef: {
				flex: 1,
				minWidth: 120,
				editable: true,
				// A case is prose, so a cell has to be able to hold a newline. The editor stays the
				// height of the row until one is added, which is what the rest of this app's grids
				// look like while you type in them.
				...multilineCellColDef
			},
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
</script>

<DarkModeObserver bind:darkMode />

<div bind:this={eGui} class="h-full w-full ag-theme-alpine {darkMode ? 'ag-theme-alpine-dark' : ''}"
></div>
