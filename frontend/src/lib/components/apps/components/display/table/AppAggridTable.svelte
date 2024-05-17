<script lang="ts">
	import { GridApi, createGrid } from 'ag-grid-community'
	import { isObject, sendUserToast } from '$lib/utils'
	import { getContext, onDestroy } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../../types'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'

	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { components, type TableAction } from '$lib/components/apps/editor/component'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { deepEqual } from 'fast-equals'
	import RefreshButton from '$lib/components/apps/components/helpers/RefreshButton.svelte'
	import SyncColumnDefs from './SyncColumnDefs.svelte'

	import 'ag-grid-community/styles/ag-grid.css'
	import './theme/windmill-theme.css'

	import {
		ChevronLeft,
		ChevronRight,
		Download,
		Loader2,
		SkipBack,
		SkipForward
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'

	import AppAggridTableActions from './AppAggridTableActions.svelte'
	import { cellRendererFactory, defaultCellRenderer } from './utils'
	import Popover from '$lib/components/Popover.svelte'
	import { Button } from '$lib/components/common'

	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let actions: TableAction[] | undefined = undefined

	const context = getContext<AppViewerContext>('AppViewerContext')

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const { app, worldStore, selectedComponent, componentControl, darkMode } = context

	const rowHeights = {
		normal: 40,
		compact: 30,
		comfortable: 50
	}

	let css = initCss($app.css?.aggridcomponent, customCss)

	let result: any[] | undefined = undefined

	$: result && setValues()

	let value: any[] = Array.isArray(result)
		? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() }))
		: [{ error: 'input was not an array' }]

	let loaded = false
	async function setValues() {
		value = Array.isArray(result)
			? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() }))
			: [{ error: 'input was not an array' }]
		if (api && loaded) {
			let selected = api.getSelectedNodes()
			if (selected && selected.length > 0 && resolvedConfig?.selectFirstRowByDefault != false) {
				let data = { ...selected[0].data }
				delete data['__index']
				outputs?.selectedRow?.set(data)
			}
		}
		if (!loaded) {
			loaded = true
		}
	}

	let resolvedConfig = initConfig(
		components['aggridcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: {},
		selectedRows: [] as any[],
		result: [] as any[],
		loading: false,
		page: 0,
		newChange: { row: 0, column: '', value: undefined, old: undefined },
		ready: undefined as boolean | undefined,
		inputs: {},
		filters: {},
		displayedRowCount: 0
	})

	let selectedRowIndex = -1

	function toggleRow(row: any) {
		if (row) {
			let rowIndex = row.rowIndex
			let data = { ...row.data }
			delete data['__index']
			if (selectedRowIndex !== rowIndex) {
				selectedRowIndex = rowIndex
				outputs?.selectedRowIndex.set(rowIndex)
			}

			if (!deepEqual(outputs?.selectedRow?.peak(), data)) {
				outputs?.selectedRow.set(data)
			}

			if (iterContext && listInputs) {
				listInputs.set(id, { selectedRow: data, selectedRowIndex: selectedRowIndex })
			}
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	function toggleRows(rows: any[]) {
		if (rows.length === 0) {
			outputs?.selectedRows.set([])
		}
		toggleRow(rows[0])
		outputs?.selectedRows.set(
			rows.map((x) => {
				let data = { ...x.data }
				delete data['__index']
				return data
			})
		)
	}

	$: outputs?.result?.set(result ?? [])

	let clientHeight
	let clientWidth

	function onCellValueChanged(event) {
		if (result) {
			let dataCell = event.newValue
			try {
				dataCell = JSON.parse(dataCell)
			} catch (e) {}
			outputs?.newChange?.set({
				row: event.node.rowIndex,
				column: event.colDef.field,
				value: dataCell,
				old: result[Number(event.node.data['__index'])][event.colDef.field]
			})
			result[Number(event.node.data['__index'])][event.colDef.field] = dataCell
			let data = { ...result[event.node.rowIndex] }
			outputs?.selectedRow?.set(data)
		}
	}

	let extraConfig = resolvedConfig.extraConfig
	let api: GridApi<any> | undefined = undefined
	let eGui: HTMLDivElement
	let state: any = undefined

	$: loaded && eGui && mountGrid()

	function refreshActions(actions: TableAction[]) {
		if (!deepEqual(actions, lastActions)) {
			lastActions = [...actions]

			updateOptions()
		}
	}

	let lastActions: TableAction[] | undefined = undefined
	$: actions && refreshActions(actions)

	let inputs = {}

	const tableActionsFactory = cellRendererFactory((c, p) => {
		const rowIndex = p.node.rowIndex ?? 0
		const row = p.data

		new AppAggridTableActions({
			target: c.eGui,
			props: {
				id: id,
				actions,
				rowIndex,
				row,
				render,
				wrapActions: resolvedConfig.wrapActions,
				selectRow: () => {
					toggleRow(p)
					p.node.setSelected(true)
				},
				onSet: (id, value) => {
					if (!inputs[id]) {
						inputs[id] = { [rowIndex]: value }
					} else {
						inputs[id] = { ...inputs[id], [rowIndex]: value }
					}

					outputs?.inputs.set(inputs, true)
				},
				onRemove: (id) => {
					if (inputs?.[id] == undefined) {
						return
					}
					delete inputs[id][rowIndex]
					inputs[id] = { ...inputs[id] }
					if (Object.keys(inputs?.[id] ?? {}).length == 0) {
						delete inputs[id]
						inputs = { ...inputs }
					}
					outputs?.inputs.set(inputs, true)
				}
			},
			context: new Map([['AppViewerContext', context]])
		})
	})

	function mountGrid() {
		if (eGui) {
			try {
				let columnDefs =
					Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
						? [...resolvedConfig?.columnDefs] // Clone to avoid direct mutation
						: []

				// Add the action column if actions are defined
				if (actions && actions.length > 0) {
					columnDefs.push({
						headerName: 'Actions',
						cellRenderer: tableActionsFactory,
						autoHeight: true,
						cellStyle: { textAlign: 'center' },
						cellClass: 'grid-cell-centered',
						...(!resolvedConfig?.wrapActions ? { minWidth: 130 * actions?.length } : {})
					})
				}

				createGrid(
					eGui,
					{
						rowData: value,
						columnDefs: columnDefs.map((fields) => {
							let cr = defaultCellRenderer(fields.cellRendererType)
							return {
								...fields,
								...(cr ? { cellRenderer: cr } : {})
							}
						}),
						pagination: resolvedConfig?.pagination,
						paginationAutoPageSize: resolvedConfig?.pagination,
						suppressPaginationPanel: true,

						defaultColDef: {
							flex: resolvedConfig.flex ? 1 : 0,
							editable: resolvedConfig?.allEditable,
							onCellValueChanged
						},
						rowHeight: resolvedConfig.compactness
							? rowHeights[resolvedConfig.compactness]
							: rowHeights['normal'],
						rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
						rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
							? resolvedConfig.rowMultiselectWithClick
							: undefined,
						onPaginationChanged: (event) => {
							outputs?.page.set(event.api.paginationGetCurrentPage())
						},
						initialState: state,
						suppressRowDeselection: true,
						suppressDragLeaveHidesColumns: true,
						...(resolvedConfig?.extraConfig ?? {}),
						onStateUpdated: (e) => {
							state = e?.api?.getState()
							resolvedConfig?.extraConfig?.['onStateUpdated']?.(e)
						},

						onGridReady: (e) => {
							outputs?.ready.set(true)
							value = value
							if (result && result.length > 0 && resolvedConfig?.selectFirstRowByDefault === true) {
								e.api.getRowNode('0')?.setSelected(true)
							}
							$componentControl[id] = {
								agGrid: { api: e.api, columnApi: e.columnApi },
								setSelectedIndex: (index) => {
									if(index === -1) {
										e.api.deselectAll();
										outputs?.selectedRow?.set({})
										outputs?.selectedRowIndex.set(-1)
									}
									e.api.getRowNode(index.toString())?.setSelected(true)
								}
							}
							api = e.api
							resolvedConfig?.extraConfig?.['onGridReady']?.(e)
						},
						onSelectionChanged: (e) => {
							onSelectionChanged(e.api)
							resolvedConfig?.extraConfig?.['onSelectionChanged']?.(e)
						},
						onCellEditingStarted: (e) => {
							e.api.getRowNode(e.data['__index'])?.setSelected(true)
						},
						onFilterChanged: (e) => {
							outputs?.filters?.set(e.api.getFilterModel())
							outputs?.displayedRowCount?.set(e.api.getDisplayedRowCount())
							resolvedConfig?.extraConfig?.['onFilterChanged']?.(e)
						},
						getRowId: (data) => data.data['__index']
					},
					{}
				)
			} catch (e) {
				console.error(e)
				sendUserToast("Couldn't mount the grid:" + e, true)
			}
		}
	}

	$: resolvedConfig && updateOptions()
	$: value && updateValue()

	$: if (!deepEqual(extraConfig, resolvedConfig.extraConfig)) {
		extraConfig = resolvedConfig.extraConfig
		if (extraConfig) {
			api?.updateGridOptions(extraConfig)
		}
	}

	function onSelectionChanged(api: GridApi<any>) {
		if (resolvedConfig?.multipleSelectable) {
			const rows = api.getSelectedNodes()
			if (rows != undefined) {
				toggleRows(rows)
			}
		} else {
			const row = api.getSelectedNodes()?.[0]
			if (row != undefined) {
				toggleRow(row)
			}
		}
	}

	function updateValue() {
		api?.updateGridOptions({ rowData: value })

		const displayedRowCount = api?.getDisplayedRowCount()
		if (displayedRowCount) {
			outputs?.displayedRowCount?.set(displayedRowCount)
		}

		if (api) {
			onSelectionChanged(api)
		}
	}

	function updateOptions() {
		try {
			const columnDefs =
				Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
					? [...resolvedConfig?.columnDefs] // Clone to avoid direct mutation
					: []

			// Add the action column if actions are defined
			if (actions && actions.length > 0) {
				columnDefs.push({
					headerName: 'Actions',
					cellRenderer: tableActionsFactory,
					autoHeight: true,
					cellStyle: { textAlign: 'center' },
					cellClass: 'grid-cell-centered',
					...(!resolvedConfig?.wrapActions ? { minWidth: 130 * actions?.length } : {})
				})
			}

			api?.updateGridOptions({
				rowData: value,
				columnDefs: columnDefs.map((fields) => {
					let cr = defaultCellRenderer(fields.cellRendererType)
					return {
						...fields,
						...(cr ? { cellRenderer: cr } : {})
					}
				}),
				pagination: resolvedConfig?.pagination,
				paginationAutoPageSize: resolvedConfig?.pagination,
				suppressPaginationPanel: true,
				suppressDragLeaveHidesColumns: true,
				defaultColDef: {
					flex: resolvedConfig.flex ? 1 : 0,
					editable: resolvedConfig?.allEditable,
					onCellValueChanged
				},
				rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
				rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
					? resolvedConfig.rowMultiselectWithClick
					: undefined,
				rowHeight: resolvedConfig.compactness
					? rowHeights[resolvedConfig.compactness]
					: rowHeights['normal'],
				...(resolvedConfig?.extraConfig ?? {})
			})
		} catch (e) {
			console.error(e)
			sendUserToast("Couldn't update the grid:" + e, true)
		}
	}
	let loading = false
	let refreshCount: number = 0
</script>

{#each Object.keys(components['aggridcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.tablecomponent}
	/>
{/each}

<RunnableWrapper
	{outputs}
	{render}
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
	hideRefreshButton={true}
>
	<SyncColumnDefs {id} columnDefs={resolvedConfig.columnDefs} {result}>
		<div
			class={twMerge(
				'flex flex-col h-full component-wrapper divide-y',
				css?.container?.class,
				'wm-aggrid-container'
			)}
			style={css?.container?.style}
			bind:clientHeight
			bind:clientWidth
		>
			{#if componentInput?.type === 'runnable' && componentInput.autoRefresh}
				<div class="absolute top-2 right-2 z-50">
					<RefreshButton {id} {loading} />
				</div>
			{/if}

			<div
				on:pointerdown|stopPropagation={() => {
					$selectedComponent = [id]
				}}
				style:height="{clientHeight}px"
				style:width="{clientWidth}px"
				class="ag-theme-alpine relative"
				class:ag-theme-alpine-dark={$darkMode}
			>
				{#key resolvedConfig?.pagination}
					{#if loaded}
						<div bind:this={eGui} style:height="100%" />
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				{/key}
			</div>
			{#if resolvedConfig.footer}
				<div class="flex gap-1 w-full justify-between items-center text-sm text-secondary/80 p-2">
					<div>
						<Popover>
							<svelte:fragment slot="text">Download</svelte:fragment>
							<Button
								startIcon={{ icon: Download }}
								color="light"
								size="xs2"
								on:click={() => {
									api?.exportDataAsCsv()
								}}
								iconOnly
							/>
						</Popover>
					</div>
					<div class="flex flex-row gap-1 items-center">
						{#if resolvedConfig?.pagination}
							{#key refreshCount}
								<div class="text-xs mx-2 text-primary">
									{(api?.paginationGetPageSize() ?? 0) * (api?.paginationGetCurrentPage() ?? 0) + 1}
									to {Math.min(
										api?.paginationGetRowCount() ?? 0,
										((api?.paginationGetCurrentPage() ?? 0) + 1) *
											(api?.paginationGetPageSize() ?? 0)
									)}
									of {api?.paginationGetRowCount()}
								</div>

								<Button
									iconOnly
									startIcon={{ icon: SkipBack }}
									color="light"
									size="xs2"
									disabled={api?.paginationGetCurrentPage() == 0}
									on:click={() => {
										api?.paginationGoToFirstPage()
										refreshCount++
									}}
								/>
								<Button
									iconOnly
									startIcon={{ icon: ChevronLeft }}
									color="light"
									size="xs2"
									disabled={api?.paginationGetCurrentPage() == 0}
									on:click={() => {
										api?.paginationGoToPreviousPage()
										refreshCount++
									}}
								/>
								<div class="text-xs mx-2 text-primary">
									Page {(api?.paginationGetCurrentPage() ?? 0) + 1} of {api?.paginationGetTotalPages() ??
										0}
								</div>
								<Button
									iconOnly
									startIcon={{ icon: ChevronRight }}
									color="light"
									size="xs2"
									disabled={(api?.paginationGetCurrentPage() ?? 0) + 1 ==
										api?.paginationGetTotalPages()}
									on:click={() => {
										api?.paginationGoToNextPage()
										refreshCount++
									}}
								/>
								<Button
									iconOnly
									startIcon={{ icon: SkipForward }}
									color="light"
									size="xs2"
									disabled={(api?.paginationGetCurrentPage() ?? 0) + 1 ==
										api?.paginationGetTotalPages()}
									on:click={() => {
										api?.paginationGoToLastPage()
										refreshCount++
									}}
								/>
							{/key}
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</SyncColumnDefs>
</RunnableWrapper>

<style>
	.ag-theme-alpine {
		--ag-row-border-style: solid;
		--ag-border-color: rgb(209 213 219);
		--ag-header-border-style: solid;
		--ag-border-radius: 0;
		--ag-alpine-active-color: #d1d5db;
	}

	.ag-theme-alpine-dark {
		--ag-border-color: #4b5563;
		--ag-alpine-active-color: #64748b;
	}
</style>
