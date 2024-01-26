<script lang="ts">
	import { GridApi, createGrid, type IDatasource } from 'ag-grid-community'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { twMerge } from 'tailwind-merge'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { HelpersService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'

	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

	let selectedRowIndex = -1
	export let s3resource: string

	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			try {
				const searchCol = params.filterModel ? Object.keys(params.filterModel)?.[0] : undefined
				const res = await HelpersService.loadParquetPreview({
					workspace: $workspaceStore!,
					path: s3resource,
					offset: params.startRow,
					limit: params.endRow - params.startRow,
					sortCol: params.sortModel?.[0]?.colId,
					sortDesc: params.sortModel?.[0]?.sort == 'desc',
					searchCol: searchCol,
					searchTerm: searchCol ? params.filterModel?.[searchCol]?.filter : undefined
				})

				const data: any[] = []

				res.columns.forEach((c) => {
					c.values.forEach((v, i) => {
						if (data[i] == undefined) {
							data.push({ __index: params.startRow + i })
						}
						if (!$enterpriseLicense && params.startRow + i > 3) {
							data[i][c.name] = 'Require EE'
						} else {
							data[i][c.name] = v
						}
					})
				})

				params.successCallback(data)
			} catch (e) {
				console.error(e)
				params.failCallback()
			}
		}
	}

	function toggleRow(row: any) {
		if (row) {
			let rowIndex = row.rowIndex
			let data = { ...row.data }
			delete data['__index']
			if (selectedRowIndex !== rowIndex) {
				selectedRowIndex = rowIndex
			}
		}
	}

	function toggleRows(rows: any[]) {
		toggleRow(rows[0])
	}

	let eGui: HTMLDivElement

	$: eGui && mountGrid()

	async function mountGrid() {
		if (eGui) {
			const res = await HelpersService.loadParquetPreview({
				workspace: $workspaceStore!,
				path: s3resource,
				limit: 0
			})

			createGrid(
				eGui,
				{
					rowModelType: 'infinite',
					datasource,
					// @ts-ignore
					columnDefs: res.columns.map((c) => {
						return {
							field: c.name,
							sortable: true,
							filter: true,
							filterParams: {
								filterOptions: ['contains'],
								maxNumConditions: 1
							}
						}
					}),
					pagination: false,
					// defaultColDef: {
					// 	flex: 1
					// },
					suppressColumnMoveAnimation: true,
					rowSelection: 'multiple',
					rowMultiSelectWithClick: true,
					suppressRowDeselection: true,

					onSelectionChanged: (e) => {
						onSelectionChanged(e.api)
					},
					getRowId: (data) => {
						return (data as any).data['__index']
					}
				},
				{}
			)
		}
	}

	function onSelectionChanged(api: GridApi<any>) {
		const rows = api.getSelectedNodes()
		if (rows != undefined) {
			toggleRows(rows)
		}
	}

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<div class={twMerge('mt-4 border shadow-sm divide-y flex flex-col h-full min-h-[600px]')}>
	<div class="ag-theme-alpine h-full" class:ag-theme-alpine-dark={darkMode}>
		<div bind:this={eGui} style="height:100%" />
	</div>
</div>
<!-- <div class="flex gap-1 absolute bottom-1 right-2 text-sm text-secondary"
	>{firstRow}{'->'}{lastRow + 1} of {datasource?.rowCount} rows</div
> -->
