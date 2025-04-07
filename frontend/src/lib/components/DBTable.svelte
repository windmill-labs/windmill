<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createGrid, type GridApi, type IDatasource } from 'ag-grid-community'
	import {
		type DbType,
		type TableMetadata,
		getPrimaryKeys,
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
			let lastRow = -1
			if (datasource?.rowCount && datasource.rowCount <= params.endRow) {
				lastRow = datasource.rowCount
			}

			if (!$workspaceStore) {
				return params.failCallback()
			}

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
					language: resourceType as ScriptLang, // TODO: Remove after updating DbType
					content: query
				}
			})) as unknown[]
			if (resourceType === 'ms_sql_server') items = items?.[0] as unknown[]
			if (!items || !Array.isArray(items)) {
				return params.failCallback()
			}
			let processedData = items.map((item: any) => {
				let primaryKeys = getPrimaryKeys(tableMetadata)
				let o = {}
				primaryKeys.forEach((pk) => (o[pk] = item[pk]))
				item['__index'] = JSON.stringify(o)
				return item
			})

			if (items.length < params.endRow - params.startRow) {
				lastRow = params.startRow + items.length
			}

			params.successCallback(processedData, lastRow)
		}
	}

	$effect(() => eGui && tableMetadata && mountGrid())
	function mountGrid() {
		if (eGui) {
			createGrid(
				eGui,
				{
					rowModelType: 'infinite',
					datasource,
					columnDefs: transformColumnDefs({
						columnDefs: tableMetadata ?? []
					}),
					pagination: false,
					defaultColDef: {
						// flex: resolvedConfig.flex ? 1 : 0,
						// editable: resolvedConfig?.allEditable,
						// onCellValueChanged
					},
					infiniteInitialRowCount: 100,
					cacheBlockSize: 100,
					cacheOverflowSize: 10,
					maxBlocksInCache: 20,
					// ...(resolvedConfig?.wrapActions
					// 	? { rowHeight: Math.max(44, actions.length * 48) }
					// 	: { rowHeight: 44 }),
					suppressColumnMoveAnimation: true,
					suppressDragLeaveHidesColumns: true,
					// rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
					// rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
					// 	? resolvedConfig.rowMultiselectWithClick
					// 	: false,
					// initialState: state,
					// suppressRowDeselection: true,
					// enableCellTextSelection: true,
					// ...(resolvedConfig?.extraConfig ?? {}),
					// onViewportChanged: (e) => {
					// 	firstRow = e.firstRow
					// 	lastRow = e.lastRow
					// },
					// onStateUpdated: (e) => {
					// 	state = e?.api?.getState()
					// 	resolvedConfig?.extraConfig?.['onStateUpdated']?.(e)
					// },
					onGridReady: (e) => {
						// outputs?.ready.set(true)
						// $componentControl[id] = {
						// 	agGrid: { api: e.api, columnApi: e.columnApi },
						// 	setSelectedIndex: (index) => {
						// 		e.api.getRowNode(index.toString())?.setSelected(true)
						// 	},
						// 	recompute: () => {
						// 		dispatch('recompute')
						// 	}
						// }
						api = e.api
						// resolvedConfig?.extraConfig?.['onGridReady']?.(e)
					}
					// onSelectionChanged: (e) => {
					// 	onSelectionChanged(e.api)
					// 	resolvedConfig?.extraConfig?.['onSelectionChanged']?.(e)
					// },
					// getRowId: (data) =>
					// 	resolvedConfig?.rowIdCol && resolvedConfig?.rowIdCol != ''
					// 		? data.data?.[resolvedConfig?.rowIdCol]
					// 		: (data.data?.['id'] ?? (data as any).data['__index'])
				},
				{}
			)
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
