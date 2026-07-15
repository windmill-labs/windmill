<script lang="ts">
	import { run } from 'svelte/legacy'

	import { type GridApi, createGrid, type IDatasource } from 'ag-grid-community'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { twMerge } from 'tailwind-merge'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { AppService, HelpersService } from '$lib/gen'
	import { base } from '$lib/base'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { Download } from 'lucide-svelte'
	import { Loader2 } from 'lucide-svelte'

	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

	let selectedRowIndex = -1
	interface Props {
		s3resource: string
		storage: string | undefined
		workspaceId: string | undefined
		disable_download?: boolean
		// When set (deployed app view), read the file on-behalf of the app author
		// through the app-scoped, provenance-gated `apps_u/*` endpoints instead of
		// the viewer-scoped `job_helpers/*` API. Undefined in the editor/preview.
		appPath?: string | undefined
	}

	let {
		s3resource,
		storage,
		workspaceId,
		disable_download = false,
		appPath = undefined
	}: Props = $props()

	// Route the parquet/csv read through the app-scoped endpoints when `appPath`
	// is set, else the viewer-scoped helpers. Same request/response shape either
	// way — the only difference is which identity authorizes the S3 read.
	function loadRowCount(searchCol: string | undefined, searchTerm: string | undefined) {
		const workspace = workspaceId ?? $workspaceStore!
		return appPath
			? AppService.appLoadTableCount({
					workspace,
					path: appPath,
					fileKey: s3resource,
					searchCol,
					searchTerm,
					storage
				})
			: HelpersService.loadTableRowCount({
					workspace,
					path: s3resource,
					searchCol,
					searchTerm,
					storage
				})
	}

	function loadChunk(args: {
		offset?: number
		limit?: number
		sortCol?: string
		sortDesc?: boolean
		searchCol?: string
		searchTerm?: string
		csvSeparator?: string
	}) {
		const workspace = workspaceId ?? $workspaceStore!
		const csv = s3resource.endsWith('.csv')
		if (appPath) {
			const data = { workspace, path: appPath, fileKey: s3resource, storage, ...args }
			return csv ? AppService.appLoadCsvPreview(data) : AppService.appLoadParquetPreview(data)
		}
		const data = { workspace, path: s3resource, storage, ...args }
		return csv ? HelpersService.loadCsvPreview(data) : HelpersService.loadParquetPreview(data)
	}

	let lastSearch: string | undefined = undefined

	let nbRows: number | undefined = $state(undefined)
	let csvSeparatorChar: string = $state(',')
	let datasource: IDatasource = {
		rowCount: 0,
		getRows: async function (params) {
			try {
				const searchCol = params.filterModel ? Object.keys(params.filterModel)?.[0] : undefined
				const searchTerm = searchCol ? params.filterModel?.[searchCol]?.filter : undefined
				const csv = s3resource.endsWith('.csv')
				const newSearch = searchCol ? searchCol + searchTerm : undefined
				if (!nbRows || lastSearch != newSearch) {
					nbRows = undefined
					const res = await loadRowCount(searchCol, searchTerm)
					nbRows = res.count
					lastSearch = newSearch
				}

				const res = (await loadChunk({
					offset: params.startRow,
					limit: params.endRow - params.startRow,
					sortCol: params.sortModel?.[0]?.colId,
					sortDesc: params.sortModel?.[0]?.sort == 'desc',
					searchCol,
					searchTerm,
					csvSeparator: csv ? csvSeparatorChar : undefined
				})) as any
				for (let i = 0; i < res.rows.length; i++) {
					res.rows[i]['__index'] = i + params.startRow
					if (!$enterpriseLicense) {
						Object.keys(res.rows[i]).forEach((key) => {
							if (key != '__index') {
								res.rows[i][key] = 'Require EE'
							}
						})
					}
				}

				params.successCallback(res.rows, nbRows)
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

	let eGui: HTMLDivElement | undefined = $state()

	let error: string | undefined = $state(undefined)
	async function mountGrid() {
		if (eGui) {
			try {
				const csv = s3resource.endsWith('.csv')

				const res = (await loadChunk({
					limit: 0,
					csvSeparator: csv ? csvSeparatorChar : undefined
				})) as any

				createGrid(
					eGui,
					{
						rowModelType: 'infinite',
						datasource,
						// @ts-ignore
						columnDefs: res.columns.map((c) => {
							return {
								field: c,
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
				error = undefined
			} catch (e) {
				error = e.body
				console.error(e)
			}
		}
	}

	function onSelectionChanged(api: GridApi<any>) {
		const rows = api.getSelectedNodes()
		if (rows != undefined) {
			toggleRows(rows)
		}
	}

	let darkMode: boolean = $state(false)
	run(() => {
		eGui && mountGrid()
	})
</script>

<DarkModeObserver bind:darkMode />

<div class={twMerge('mt-2  flex flex-col h-full min-h-[600px]')}>
	{#if s3resource.endsWith('.csv')}
		<div class="flex flex-row-reverse w-full">
			<div class="flex items-baseline">
				<label for="csvSeparatorChar" class="text-2xs text-secondary">Separator</label>

				<div class="w-12 ml-2 mr-2">
					<select class="h-8" bind:value={csvSeparatorChar} onchange={(e) => mountGrid()}>
						<option value=",">,</option>
						<option value=";">;</option>
						<option value="\t">\t</option>
						<option value="|">|</option>
					</select>
				</div>
			</div>
		</div>
	{/if}
	{#if !disable_download && !s3resource.endsWith('.csv')}
		{@const csvApiPath = appPath
			? `/w/${workspaceId}/apps_u/download_s3_parquet_file_as_csv/${appPath}?file_key=${encodeURIComponent(s3resource)}${storage ? `&storage=${storage}` : ''}`
			: `/w/${workspaceId}/job_helpers/download_s3_parquet_file_as_csv?file_key=${encodeURIComponent(s3resource)}${storage ? `&storage=${storage}` : ''}`}
		{@const csvName = (s3resource.split('/').pop() ?? 'download') + '.csv'}
		{#if shouldDownloadViaClient()}
			<button
				class="text-secondary w-full text-right underline text-2xs whitespace-nowrap"
				onclick={() => downloadViaClient(csvApiPath, csvName)}
				><div class="flex flex-row-reverse gap-2 items-center"><Download size={12} /> CSV</div
				></button
			>
		{:else}
			<a
				target="_blank"
				href="{base}/api{csvApiPath}"
				class="text-secondary w-full text-right underline text-2xs whitespace-nowrap"
				><div class="flex flex-row-reverse gap-2 items-center"><Download size={12} /> CSV</div></a
			>
		{/if}
	{/if}

	{#if nbRows != undefined}
		<div class="text-secondary ml-0.5 text-2xs">{nbRows} rows</div>
	{:else}
		<Loader2 class="animate-spin ml-0.5" size={12} />
	{/if}
	<div
		class="ag-theme-alpine shadow-sm h-full"
		class:ag-theme-alpine-dark={darkMode}
		style="height: 600px;"
	>
		{#if error}
			<div class="text-red-500">{error}</div>
			<div>Try changing separator to fix it</div>
		{/if}
		<div bind:this={eGui} style="height:100%; "></div>
	</div>
</div>

<!-- <div class="flex gap-1 absolute bottom-1 right-2 text-sm text-secondary"
	>{firstRow}{'->'}{lastRow + 1} of {datasource?.rowCount} rows</div
> -->
