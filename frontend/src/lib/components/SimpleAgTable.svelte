<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { createGrid, type GridApi } from 'ag-grid-community'
	import DarkModeObserver from './DarkModeObserver.svelte'

	type Props = {
		data: Record<string, any>[]
		class?: string
	}
	let { data, class: className }: Props = $props()
	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()

	$effect(() => eGui && mountGrid())
	function mountGrid() {
		if (eGui) {
			createGrid(eGui, {
				onGridReady: (e) => {
					api = e.api
				},
				autoSizeStrategy: { type: 'fitCellContents' },
				...updateGridOptions
			})
		}
	}

	let columnDefs = $derived.by(() => {
		if (!data.length) return []
		const defs: { field: string }[] = []
		for (const row of data) {
			for (const key in row) {
				if (!defs.some((def) => def.field === key)) {
					defs.push({ field: key })
				}
			}
		}
		return defs
	})

	let updateGridOptions = $derived({
		rowData: data.map((row) => {
			const newRow: Record<string, any> = {}
			for (const key in row) {
				newRow[key] = typeof row[key] === 'string' ? row[key].trim() : JSON.stringify(row[key])
			}
			return newRow
		}),
		columnDefs
	})

	$effect(() => {
		api?.updateGridOptions(updateGridOptions)
	})

	let darkMode = $state(false)
	let [clientHeight, clientWidth] = $state([0, 0])
</script>

<DarkModeObserver bind:darkMode />

<div
	class={'flex flex-col h-full component-wrapper divide-y wm-aggrid-container ' +
		(api ? '' : 'opacity-0 ') +
		className}
	bind:clientHeight
	bind:clientWidth
>
	<div
		style:height="{clientHeight}px"
		style:width="{clientWidth}px"
		class="ag-theme-alpine"
		class:ag-theme-alpine-dark={darkMode}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			bind:this={eGui}
			style:height="100%"
			onkeydown={(e) => {
				if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
					const selectedCell = api?.getFocusedCell()
					if (selectedCell) {
						const rowIndex = selectedCell.rowIndex
						const colId = selectedCell.column?.getId()
						const rowNode = api?.getDisplayedRowAtIndex(rowIndex)
						const selectedValue = rowNode?.data?.[colId]
						navigator.clipboard.writeText(selectedValue)
						sendUserToast('Copied cell value to clipboard', false)
					}
				}
			}}
		>
		</div>
	</div>
</div>
