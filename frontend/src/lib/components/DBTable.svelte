<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createGrid, type GridApi, type IDatasource } from 'ag-grid-community'
	import { transformColumnDefs } from './apps/components/display/table/utils'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button } from './common'
	import { Download } from 'lucide-svelte'
	import Popover from './Popover.svelte'
	import DebouncedInput from './apps/components/helpers/DebouncedInput.svelte'
	import InsertRowDrawerButton from './apps/components/display/InsertRowDrawerButton.svelte'
	import type { IDbTableOps } from './dbOps'
	import { deepEqual } from 'fast-equals'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import '$lib/components/apps/components/display/table/theme/windmill-theme.css'
	import { untrack } from 'svelte'

	type Props = {
		dbTableOps: IDbTableOps
	}
	let { dbTableOps }: Props = $props()

	let [clientHeight, clientWidth, darkMode, firstRow, lastRow] = $state([0, 0, false, -1, -1])
	let quicksearch = $state('')
	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()

	let datasource: IDatasource = {
		getRows: async function (params) {
			if (!$workspaceStore) return params.failCallback()
			let lastRow = rowCount && rowCount <= params.endRow ? rowCount : -1

			const items = await dbTableOps.getRows({
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch: params.context.quicksearch,
				order_by: params.sortModel?.[0]?.colId ?? dbTableOps.colDefs[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			})

			if (items.length < params.endRow - params.startRow) lastRow = params.startRow + items.length
			params.successCallback(items, lastRow)
		}
	}
	let rowCount = $state(0)
	let refreshCount = $state(0)
	const refresh = () => (refreshCount += 1)

	$effect(() => eGui && untrack(() => mountGrid()))
	function mountGrid() {
		if (eGui && !api) {
			createGrid(eGui, {
				rowModelType: 'infinite',
				pagination: false,
				...(dbTableOps.onUpdate && {
					defaultColDef: {
						flex: 1,
						minWidth: 160,
						editable: true,
						onCellValueChanged: (e) => {
							if (!$workspaceStore) return
							const colDef = e.colDef as unknown as { field: string; datatype: string }
							dbTableOps
								.onUpdate?.(
									{ values: { ...(e.data as object), [colDef.field]: e.oldValue } },
									colDef,
									e.newValue
								)
								.then(() => {
									sendUserToast('Value updated')
								})
								.catch(() => {
									sendUserToast('Error updating value', true)
									refresh?.()
								})
						}
					}
				}),
				onViewportChanged: (e) => ([firstRow, lastRow] = [e.firstRow, e.lastRow]),
				infiniteInitialRowCount: 100,
				cacheBlockSize: 100,
				cacheOverflowSize: 10,
				maxBlocksInCache: 20,
				suppressColumnMoveAnimation: true,
				suppressDragLeaveHidesColumns: true,
				onGridReady: (e) => {
					api = e.api
				}
			})
		}
	}

	let prevUpdateKey: any = undefined
	$effect(() => {
		if (!$workspaceStore || !api) return
		const key = { quicksearch, colDefs: dbTableOps.colDefs, refreshCount }
		if (deepEqual(key, prevUpdateKey)) return
		prevUpdateKey = key
		untrack(() => updateGrid())
	})
	function updateGrid() {
		dbTableOps.getCount({ quicksearch }).then((result) => (rowCount = result))

		api?.purgeInfiniteCache()
		api?.updateGridOptions({
			datasource,
			columnDefs: transformColumnDefs({
				columnDefs: dbTableOps.colDefs ?? [],
				...(dbTableOps.onDelete && {
					onDelete: (values) => {
						if (!$workspaceStore) return
						dbTableOps
							.onDelete?.({ values })
							.then(() => {
								refresh?.()
								sendUserToast('Row deleted')
							})
							.catch(() => {
								sendUserToast('Error deleting row', true)
							})
					}
				})
			}),
			context: {
				quicksearch
			}
		})
	}
</script>

<DarkModeObserver bind:darkMode />

<div class="h-full relative flex flex-col">
	<div class="flex py-2 h-12 justify-between gap-4">
		<DebouncedInput
			class="w-full max-w-[300px]"
			type="text"
			bind:value={quicksearch}
			placeholder="Search..."
		/>
		{#if dbTableOps.onInsert}
			<InsertRowDrawerButton
				columnDefs={dbTableOps.colDefs ?? []}
				dbType={dbTableOps.resourceType}
				onInsert={(values) => {
					if (!$workspaceStore) return
					dbTableOps.onInsert?.({ values }).then((result) => {
						refresh?.()
						sendUserToast('Row inserted')
					})
				}}
			/>
		{/if}
	</div>
	<div
		class={'flex flex-col flex-1 component-wrapper divide-y wm-aggrid-container'}
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
				class={api ? '' : 'opacity-0'}
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
			></div>
		</div>

		<div class="flex gap-1 w-full justify-between items-center text-xs text-primary p-2">
			<div>
				<Popover>
					{#snippet text()}
						Download
					{/snippet}
					<Button
						startIcon={{ icon: Download }}
						color="light"
						size="xs2"
						on:click={() => api?.exportDataAsCsv()}
						iconOnly
					/>
				</Popover>
			</div>
			{#if rowCount}
				{firstRow}{'->'}{lastRow + 1} of {rowCount} rows
			{:else}
				{firstRow}{'->'}{lastRow + 1}
			{/if}
		</div>
	</div>
</div>
