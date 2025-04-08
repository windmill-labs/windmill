<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createGrid, type GridApi, type IDatasource } from 'ag-grid-community'
	import {
		type DbType,
		type TableMetadata,
		loadTableMetaData
	} from './apps/components/display/dbtable/utils'
	import { transformColumnDefs } from './apps/components/display/table/utils'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { makeSelectQuery } from './apps/components/display/dbtable/queries/select'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import type { ScriptLang } from '$lib/gen'
	import { Button } from './common'
	import { Download, Plus } from 'lucide-svelte'
	import Popover from './Popover.svelte'
	import { makeCountQuery } from './apps/components/display/dbtable/queries/count'
	import DebouncedInput from './apps/components/helpers/DebouncedInput.svelte'
	import { makeUpdateQuery } from './apps/components/display/dbtable/queries/update'

	type Props = {
		resourceType: DbType
		resourcePath: string
		tableKey: string // Can contain schema prefix
	}

	let tableMetadata: TableMetadata | undefined = $state()
	$effect(() => {
		const currSelected = tableKey
		tableMetadata = undefined
		loadTableMetaData('$res:' + resourcePath, $workspaceStore, tableKey, resourceType).then(
			(tm) => tableKey === currSelected && (tableMetadata = tm)
		)
	})

	let [clientHeight, clientWidth, darkMode, firstRow, lastRow] = $state([0, 0, false, -1, -1])
	let quicksearch = $state('')
	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()

	let datasource: IDatasource = {
		getRows: async function (params) {
			if (!$workspaceStore || !tableMetadata) return params.failCallback()

			let lastRow = rowCount && rowCount <= params.endRow ? rowCount : -1

			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch: params.context.quicksearch,
				order_by: params.sortModel?.[0]?.colId ?? tableMetadata?.[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			}

			const query = makeSelectQuery(tableKey, tableMetadata, undefined, resourceType as DbType)
			let items = (await runPreviewJobAndPollResult({
				workspace: $workspaceStore,
				requestBody: {
					args: { database: '$res:' + resourcePath, ...currentParams },
					language: resourceType as ScriptLang,
					content: query
				}
			})) as unknown[]
			if (resourceType === 'ms_sql_server') items = items?.[0] as unknown[]
			if (!items || !Array.isArray(items)) {
				return params.failCallback()
			}
			if (items.length < params.endRow - params.startRow) lastRow = params.startRow + items.length
			params.successCallback(items, lastRow)
		}
	}
	let rowCount = $state(0)

	$effect(() => {
		if (!tableMetadata || !$workspaceStore) return
		const countQuery = makeCountQuery(resourceType, tableKey, undefined, tableMetadata)
		runPreviewJobAndPollResult({
			workspace: $workspaceStore,
			requestBody: {
				args: { database: '$res:' + resourcePath, quicksearch },
				language: resourceType as ScriptLang,
				content: countQuery
			}
		}).then((result) => (rowCount = result?.[0].count as number))
	})

	$effect(() => eGui && mountGrid())
	function mountGrid() {
		if (eGui) {
			createGrid(eGui, {
				rowModelType: 'infinite',
				pagination: false,
				defaultColDef: {
					editable: true, // TODO: configurable
					onCellValueChanged: (e) => {
						if (!tableMetadata || !$workspaceStore) return
						const colDef = e.colDef as unknown as { field: string; datatype: string }
						const updateQuery = makeUpdateQuery(tableKey, colDef, tableMetadata, resourceType)

						runPreviewJobAndPollResult({
							workspace: $workspaceStore,
							requestBody: {
								args: {
									database: '$res:' + resourcePath,
									value_to_update: e.newValue,
									...(e.data as object),
									[colDef.field]: e.oldValue
								},
								language: resourceType as ScriptLang,
								content: updateQuery
							}
						})
							.then((result) => {
								if (!Array.isArray(result) || result.length === 0) throw ''
								sendUserToast('Value updated')
							})
							.catch(() => {
								sendUserToast('Error updating value', true)
							})
					}
				},
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

	$effect(() => {
		;[quicksearch, tableMetadata]
		updateGrid()
	})
	function updateGrid() {
		api?.updateGridOptions({
			datasource,
			columnDefs: transformColumnDefs({
				columnDefs: tableMetadata ?? []
			}),
			context: {
				quicksearch
			}
		})
	}

	let { resourcePath, resourceType, tableKey }: Props = $props()
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
		<div class="flex flex-row gap-2">
			<Button
				startIcon={{ icon: Plus }}
				color="dark"
				size="xs2"
				on:click={() => {
					// args = {}
					// insertDrawer?.openDrawer()
				}}
			>
				Insert
			</Button>
		</div>
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
					<svelte:fragment slot="text">Download</svelte:fragment>
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
