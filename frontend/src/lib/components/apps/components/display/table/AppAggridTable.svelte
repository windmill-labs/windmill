<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { type GridApi, createGrid } from 'ag-grid-community'
	import { isObject, sendUserToast } from '$lib/utils'
	import { getContext, mount, onDestroy, unmount, untrack } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ContextPanelContext,
		ListContext,
		ListInputs,
		RichConfiguration,
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
	import 'ag-grid-community/styles/ag-theme-alpine.css'
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
	import { deepCloneWithFunctions, initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'

	import AppAggridTableActions from './AppAggridTableActions.svelte'
	import { cellRendererFactory, transformColumnDefs, type WindmillColumnDef } from './utils'
	import Popover from '$lib/components/Popover.svelte'
	import { Button } from '$lib/components/common'
	import InputValue from '../../helpers/InputValue.svelte'
	import { stateSnapshot, withProps } from '$lib/svelte5Utils.svelte'
	import { get } from 'svelte/store'
	import SubGridEditor from '$lib/components/apps/editor/SubGridEditor.svelte'

	interface Props {
		// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'
		id: string
		componentInput: AppInput | undefined
		configuration: RichConfigurations
		initializing?: boolean | undefined
		render: boolean
		customCss?: ComponentCustomCSS<'aggridcomponent'> | undefined
		actions?: TableAction[] | undefined
		actionsOrder?: RichConfiguration | undefined
		onChange?: string[] | undefined
	}

	let {
		id,
		componentInput,
		configuration,
		initializing = $bindable(undefined),
		render,
		customCss = undefined,
		actions = undefined,
		actionsOrder = undefined,
		onChange = undefined
	}: Props = $props()

	const context = getContext<AppViewerContext>('AppViewerContext')
	const contextPanel = getContext<ContextPanelContext>('ContextPanel')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const editorContext = getContext('AppEditorContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const { app, worldStore, selectedComponent, componentControl, darkMode, mode } = context

	const rowHeights = {
		normal: 40,
		compact: 30,
		comfortable: 50
	}

	let css = $state(initCss($app.css?.aggridcomponent, customCss))

	let result: any[] | undefined = $state(undefined)

	function resetValues() {
		api?.setGridOption('rowData', value)
	}

	let uid = Math.random().toString(36).substring(7)
	let prevUid: string | undefined = undefined

	let value: any[] = $state(
		untrack(() =>
			Array.isArray(result)
				? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() + '-' + uid }))
				: [{ error: 'input was not an array' }]
		)
	)

	let loaded = $state(false)

	async function setValues() {
		value = Array.isArray(result)
			? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() + '-' + uid }))
			: [{ error: 'input was not an array' }]
		prevUid = uid
		uid = Math.random().toString(36).substring(7)

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

	let resolvedConfig = $state(
		initConfig(components['aggridcomponent'].initialData.configuration, configuration)
	)

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: {},
		selectedRows: [] as any[],
		openedModalRow: {},
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
				outputs?.selectedRow?.set(data)
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
		const selectedRows = rows.map((x) => {
			let data = { ...x.data }
			delete data['__index']
			return data
		})
		outputs?.selectedRows.set(selectedRows)

		if (iterContext && listInputs) {
			listInputs.set(id, { selectedRows })
		}
	}

	let clientHeight: number = $state(0)
	let clientWidth: number = $state(0)

	function fireOnChange() {
		let runnableComponents = get(context.runnableComponents)
		if (onChange) {
			onChange.forEach((id) => runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function onCellValueChanged(event) {
		if (result) {
			let dataCell = event.newValue
			try {
				dataCell = JSON.parse(dataCell)
			} catch (e) {}
			let idx = Number(event.node.data['__index'].split('-')[0])
			uid = prevUid ?? ''
			outputs?.newChange?.set({
				row: idx,
				column: event.colDef.field,
				value: dataCell,
				old: result[idx][event.colDef.field]
			})
			result[idx][event.colDef.field] = dataCell

			let data = { ...result[idx] }
			outputs?.selectedRow?.set(data)
			resolvedConfig?.extraConfig?.['defaultColDef']?.['onCellValueChanged']?.(event)
			fireOnChange()
		}
	}

	let extraConfig = $state(deepCloneWithFunctions(resolvedConfig.extraConfig))
	let api: GridApi<any> | undefined = $state(undefined)
	let eGui: HTMLDivElement | undefined = $state(undefined)
	let componentState: any = undefined

	function refreshActions(actions: TableAction[]) {
		if (!deepEqual(actions, lastActions)) {
			lastActions = structuredClone(stateSnapshot(actions))
			updateOptions()
		}
	}

	let lastActions: TableAction[] | undefined = undefined

	let lastActionsOrder: string[] | undefined = undefined

	function clearActionOrder() {
		computedOrder = undefined
		updateOptions()
	}

	function refreshActionsOrder(actionsOrder: string[] | undefined) {
		if (Array.isArray(actionsOrder) && !deepEqual(actionsOrder, lastActionsOrder)) {
			lastActionsOrder = [...actionsOrder]

			updateOptions()
		}
	}

	let inputs = {}

	const tableActionsFactory = cellRendererFactory((c, p) => {
		const rowIndex = p.node.rowIndex ?? 0
		const row = p.data

		const componentContext = new Map<string, any>([
			['AppViewerContext', context],
			['ContextPanel', contextPanel],
			['AppEditorContext', editorContext]
		])

		const sortedActions: TableAction[] | undefined = computedOrder
			? (computedOrder
					.map((key) => actions?.find((a) => a.id === key))
					.filter(Boolean) as TableAction[])
			: actions

		const taComponent = withProps(AppAggridTableActions, {
			p,
			id: id,
			actions: sortedActions,
			rowIndex,
			row,
			render,
			wrapActions: resolvedConfig.wrapActions,
			selectRow: (p) => {
				toggleRow(p)
				p.node.setSelected(true)
			},
			setModalRow: (row) => {
				const data = { ...(row?.data ?? {}) }
				delete data['__index']
				if (!deepEqual(outputs?.openedModalRow?.peak(), data)) outputs?.openedModalRow.set(data)
			},
			onSet: (id, value, rowIndex) => {
				if (!inputs[id]) {
					inputs[id] = { [rowIndex]: value }
				} else {
					inputs[id] = { ...inputs[id], [rowIndex]: value }
				}

				outputs?.inputs.set(inputs, true)
			},
			onRemove: (id, rowIndex) => {
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
		})
		let ta = mount(taComponent.component, {
			target: c.eGui,
			props: taComponent.props,
			context: componentContext
		})

		return {
			destroy: () => unmount(ta),
			refresh(params) {
				taComponent.props.rowIndex = params.node.rowIndex ?? 0
				taComponent.props.row = params.data
				taComponent.props.p = params
				const nextActions: TableAction[] | undefined = computedOrder
					? (computedOrder
							.map((key) => actions?.find((a) => a.id === key))
							.filter(Boolean) as TableAction[])
					: actions
				taComponent.props.actions = nextActions
			}
		}
	})

	function getIdFromData(data: any): string {
		return resolvedConfig?.rowIdCol && resolvedConfig?.rowIdCol != ''
			? (data?.[resolvedConfig?.rowIdCol] ?? data?.['__index'])
			: data?.['__index']
	}

	function mountGrid() {
		// console.log(resolvedConfig?.extraConfig)
		if (eGui) {
			try {
				const agColumnDefs = transformColumnDefs({
					columnDefs:
						Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
							? ([...resolvedConfig?.columnDefs] as WindmillColumnDef[])
							: [],
					actions,
					customActionsHeader: resolvedConfig?.customActionsHeader,
					wrapActions: resolvedConfig?.wrapActions,
					tableActionsFactory,
					onInvalidColumnDefs: (errors) => {
						sendUserToast(`Invalid columnDefs: ${errors.join('\n')}`, true)
					}
				})

				createGrid(
					eGui,
					{
						rowData: value,
						columnDefs: agColumnDefs,
						pagination: resolvedConfig?.pagination,
						paginationAutoPageSize: resolvedConfig?.pagination,
						suppressPaginationPanel: true,
						rowHeight: resolvedConfig.compactness
							? rowHeights[resolvedConfig.compactness]
							: rowHeights['normal'],
						rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
						rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
							? resolvedConfig.rowMultiselectWithClick
							: undefined,
						onPaginationChanged: (event) => {
							outputs?.page.set(event.api.paginationGetCurrentPage())
							footerRenderCount++
						},
						initialState: componentState,
						suppressRowDeselection: true,
						suppressDragLeaveHidesColumns: true,
						enableCellTextSelection: true,
						...deepCloneWithFunctions(resolvedConfig?.extraConfig ?? {}),
						defaultColDef: {
							flex: resolvedConfig.flex ? 1 : 0,
							editable: resolvedConfig?.allEditable,
							onCellValueChanged,
							...resolvedConfig?.extraConfig?.['defaultColDef']
						},
						onStateUpdated: (e) => {
							componentState = e?.api?.getState()
							resolvedConfig?.extraConfig?.['onStateUpdated']?.(e)
						},

						onGridReady: (e) => {
							outputs?.ready.set(true)
							value = value
							if (
								value &&
								value.length > 0 &&
								resolvedConfig?.selectFirstRowByDefault === true &&
								selectedRowIndex === -1
							) {
								e.api.getRowNode(getIdFromData(value[0]))?.setSelected(true)
							}
							$componentControl[id] = {
								agGrid: { api: e.api, columnApi: e.columnApi },
								setSelectedIndex: (index) => {
									if (index === null) {
										e.api.deselectAll()
										outputs?.selectedRow?.set({})
										outputs?.selectedRowIndex.set(0)
									} else if (Array.isArray(index)) {
										// select all rows matching the indixes
										e.api.deselectAll()
										index.forEach((i) => {
											let rowId = getIdFromData(value[i])
											if (rowId) {
												e.api.getRowNode(rowId)?.setSelected(true, false)
											}
										})
									} else if (typeof index === 'number') {
										let rowId = getIdFromData(value[index])
										if (rowId) {
											e.api.getRowNode(rowId)?.setSelected(true, true)
											outputs?.selectedRowIndex.set(index)
											const row = { ...value[index] }
											delete row['__index']
											outputs?.selectedRow?.set(row)
										}
									}
								},
								setValue(nvalue) {
									if (Array.isArray(nvalue)) {
										value = nvalue
									}
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
							e.api.getRowNode(getIdFromData(e.data))?.setSelected(true)
						},
						onFilterChanged: (e) => {
							outputs?.filters?.set(e.api.getFilterModel())
							outputs?.displayedRowCount?.set(e.api.getDisplayedRowCount())
							resolvedConfig?.extraConfig?.['onFilterChanged']?.(e)
						},
						getRowId: (data) => getIdFromData(data.data)
					},
					{}
				)
			} catch (e) {
				console.error(e)
				sendUserToast("Couldn't mount the grid:" + e, true)
			}
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
		api?.setGridOption('rowData', value)

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
			const agColumnDefs = transformColumnDefs({
				columnDefs:
					Array.isArray(resolvedConfig?.columnDefs) && resolvedConfig.columnDefs.every(isObject)
						? ([...resolvedConfig?.columnDefs] as WindmillColumnDef[])
						: [],
				actions,
				customActionsHeader: resolvedConfig?.customActionsHeader,
				wrapActions: resolvedConfig?.wrapActions,
				tableActionsFactory,
				onInvalidColumnDefs: (errors) => {
					sendUserToast(`Invalid columnDefs: ${errors.join('\n')}`, true)
				}
			})

			api?.updateGridOptions({
				rowData: value,
				columnDefs: agColumnDefs,
				pagination: resolvedConfig?.pagination,
				paginationAutoPageSize: resolvedConfig?.pagination,
				suppressPaginationPanel: true,
				suppressDragLeaveHidesColumns: true,
				rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
				rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
					? resolvedConfig.rowMultiselectWithClick
					: undefined,
				rowHeight: resolvedConfig.compactness
					? rowHeights[resolvedConfig.compactness]
					: rowHeights['normal'],
				...deepCloneWithFunctions(resolvedConfig?.extraConfig ?? {}),
				defaultColDef: {
					flex: resolvedConfig.flex ? 1 : 0,
					editable: resolvedConfig?.allEditable,
					onCellValueChanged,
					...resolvedConfig?.extraConfig?.['defaultColDef']
				}
			})
			// Force refresh to re-render cell renderers after actions change
			api?.refreshCells({ force: true })
			// Force complete redraw to clear stale inline styles
			api?.redrawRows()
		} catch (e) {
			console.error(e)
			sendUserToast("Couldn't update the grid:" + e, true)
		}
	}
	let loading = $state(false)
	let refreshCount: number = $state(0)
	let footerRenderCount: number = $state(0)
	let computedOrder: string[] | undefined = $state(undefined)

	let footerHeight: number = $state(0)
	$effect(() => {
		resolvedConfig?.rowIdCol && untrack(() => resetValues())
	})
	$effect(() => {
		result && untrack(() => setValues())
	})
	$effect(() => {
		outputs?.result?.set(result ?? [])
	})
	$effect(() => {
		loaded && eGui && untrack(() => mountGrid())
	})
	$effect(() => {
		actions && refreshActions(actions)
	})
	$effect(() => {
		computedOrder && untrack(() => refreshActionsOrder(computedOrder))
	})
	$effect(() => {
		computedOrder &&
			computedOrder.length > 0 &&
			actionsOrder === undefined &&
			untrack(() => clearActionOrder())
	})
	$effect(() => {
		api && resolvedConfig && updateOptions()
	})
	$effect(() => {
		value && untrack(() => updateValue())
	})
	$effect(() => {
		if (!deepEqual(extraConfig, resolvedConfig.extraConfig)) {
			extraConfig = deepCloneWithFunctions(resolvedConfig.extraConfig)
			if (api && extraConfig) {
				untrack(() => {
					api?.updateGridOptions(extraConfig)
				})
			}
		}
	})
</script>

{#if actionsOrder}
	<InputValue key="actionsOrder" {id} input={actionsOrder} bind:value={computedOrder} />
{/if}

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

{#if render == false || loading || initializing}
	<div class="overflow-hidden h-0">
		{#each actions?.filter((x) => x.type == 'modalcomponent') ?? [] as action}
			<SubGridEditor id={action.id} subGridId={`${action.id}-0`} />
		{/each}
	</div>
{/if}
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
	<SyncColumnDefs
		{id}
		columnDefs={resolvedConfig.columnDefs}
		{result}
		actionsPresent={Array.isArray(actions) && actions.length > 0}
		customActionsHeader={resolvedConfig?.customActionsHeader}
	>
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
				onpointerdown={stopPropagation(() => {
					$selectedComponent = [id]
				})}
				style:height="{clientHeight - (resolvedConfig.footer ? footerHeight : 0)}px"
				style:width="{clientWidth}px"
				class="ag-theme-alpine relative"
				class:ag-theme-alpine-dark={$darkMode}
			>
				{#key resolvedConfig?.pagination}
					{#if loaded}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							bind:this={eGui}
							style:height="100%"
							onkeydown={(e) => {
								if ((e.ctrlKey || e.metaKey) && e.key === 'c' && $mode !== 'dnd') {
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
					{:else}
						<Loader2 class="animate-spin" />
					{/if}
				{/key}
			</div>
			{#if resolvedConfig.footer}
				{#key footerRenderCount}
					<div
						class="flex gap-1 w-full justify-between items-center text-sm text-secondary/80 p-2"
						bind:clientHeight={footerHeight}
					>
						<div>
							<Popover>
								{#snippet text()}
									Download
								{/snippet}
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
										{(api?.paginationGetPageSize() ?? 0) * (api?.paginationGetCurrentPage() ?? 0) +
											1}
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
				{/key}
			{/if}
		</div>
	</SyncColumnDefs>
</RunnableWrapper>

<style global>
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

	.grid-cell-centered {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.grid-cell-centered .svelte-select {
		height: 32px !important;
	}

	.grid-cell-centered .selected-item {
		margin-top: -4px;
	}
</style>
