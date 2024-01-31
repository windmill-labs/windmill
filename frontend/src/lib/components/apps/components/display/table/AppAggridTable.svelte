<script lang="ts">
	import { GridApi, createGrid } from 'ag-grid-community'
	import { isObject, sendUserToast } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'

	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { components } from '$lib/components/apps/editor/component'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { deepEqual } from 'fast-equals'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined

	const { app, worldStore, selectedComponent, componentControl, darkMode } =
		getContext<AppViewerContext>('AppViewerContext')

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
		newChange: { row: 0, column: '', value: undefined },
		ready: undefined as boolean | undefined,
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
		}
	}

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
				value: dataCell
			})
			result[event.node.rowIndex][event.colDef.field] = dataCell
			let data = { ...result[event.node.rowIndex] }
			outputs?.selectedRow?.set(data)
		}
	}

	let extraConfig = resolvedConfig.extraConfig

	let api: GridApi<any> | undefined = undefined

	let eGui: HTMLDivElement

	$: loaded && eGui && mountGrid()

	let state: any = undefined
	function mountGrid() {
		if (eGui) {
			try {
				createGrid(
					eGui,
					{
						rowData: value,
						columnDefs:
							Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
								? resolvedConfig?.columnDefs
								: [],
						pagination: resolvedConfig?.pagination,
						paginationAutoPageSize: resolvedConfig?.pagination,
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
						...(resolvedConfig?.extraConfig ?? {}),
						onStateUpdated: (e) => {
							state = e?.api?.getState()
							resolvedConfig?.extraConfig?.['onStateUpdated']?.(e)
						},

						onGridReady: (e) => {
							outputs?.ready.set(true)
							value = value
							if (result && result.length > 0 && resolvedConfig?.selectFirstRowByDefault != false) {
								e.api.getRowNode('0')?.setSelected(true)
							}
							$componentControl[id] = {
								agGrid: { api: e.api, columnApi: e.columnApi },
								setSelectedIndex: (index) => {
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
			api?.updateGridOptions({
				rowData: value,
				columnDefs:
					Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
						? resolvedConfig?.columnDefs
						: undefined,
				pagination: resolvedConfig?.pagination,
				paginationAutoPageSize: resolvedConfig?.pagination,
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

<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	{#if Array.isArray(value) && value.every(isObject)}
		{#if Array.isArray(resolvedConfig.columnDefs) && resolvedConfig.columnDefs.every(isObject)}
			<div
				class={twMerge(
					'border shadow-sm divide-y flex flex-col h-full',
					css?.container?.class,
					'wm-aggrid-container'
				)}
				style={css?.container?.style}
				bind:clientHeight
				bind:clientWidth
			>
				<div
					on:pointerdown|stopPropagation={() => {
						$selectedComponent = [id]
					}}
					style:height="{clientHeight}px"
					style:width="{clientWidth}px"
					class="ag-theme-alpine"
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
			</div>
		{:else if resolvedConfig.columnDefs != undefined}
			<Alert title="Parsing issues" type="error" size="xs">
				The columnDefs should be an array of objects, received:
				<pre class="overflow-auto">
				{JSON.stringify(resolvedConfig.columnDefs)}
			</pre>
			</Alert>
		{:else}
			<Alert title="Parsing issues" type="error" size="xs">The columnDefs are undefined</Alert>
		{/if}
	{:else if result != undefined}
		<Alert title="Parsing issues" type="error" size="xs">
			The result should be an array of objects, received:
			<pre class="overflow-auto mt-2"
				>{JSON.stringify(result)}
			</pre>
		</Alert>
	{/if}
</RunnableWrapper>
