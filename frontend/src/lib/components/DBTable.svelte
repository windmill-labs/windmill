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

	let [clientHeight, clientWidth, darkMode] = $state([0, 0, false])
	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()

	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			if (!$workspaceStore) return params.failCallback()

			let lastRow =
				datasource?.rowCount && datasource.rowCount <= params.endRow ? datasource.rowCount : -1

			const currentParams = {
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch: '', // TODO
				order_by: params.sortModel?.[0]?.colId ?? tableMetadata?.[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			}

			const query = makeSelectQuery(
				tableKey,
				tableMetadata ?? [],
				undefined,
				resourceType as DbType
			)
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

	$effect(() => eGui && tableMetadata && mountGrid())
	function mountGrid() {
		if (eGui) {
			createGrid(eGui, {
				rowModelType: 'infinite',
				datasource,
				columnDefs: transformColumnDefs({
					columnDefs: tableMetadata ?? []
				}),
				pagination: false,
				defaultColDef: {
					editable: true, // TODO: configurable
					onCellValueChanged: (e) => {}
				},
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

	let { resourcePath, resourceType, tableKey }: Props = $props()
</script>

<DarkModeObserver bind:darkMode />

<div
	class={'flex flex-col h-full component-wrapper divide-y wm-aggrid-container'}
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
</div>
