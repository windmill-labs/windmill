<script lang="ts">
	import { GridApi, createGrid, type IDatasource } from 'ag-grid-community'
	import { isObject, sendUserToast } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../../types'

	import type { components } from '$lib/components/apps/editor/component'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { deepEqual } from 'fast-equals'

	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import type { Output } from '$lib/components/apps/rx'
	import type { InitConfig } from '$lib/components/apps/editor/appUtils'
	import { Button } from '$lib/components/common'
	import { cellRendererFactory } from './utils'
	import { Trash2 } from 'lucide-svelte'
	import type { ColumnDef } from '../dbtable/utils'
	// import 'ag-grid-community/dist/styles/ag-theme-alpine-dark.css'

	export let id: string
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let containerHeight: number | undefined = undefined
	export let resolvedConfig: InitConfig<
		(typeof components)['dbexplorercomponent']['initialData']['configuration']
	>
	export let datasource: IDatasource
	export let state: any = undefined
	export let outputs: Record<string, Output<any>>
	export let allowDelete: boolean

	const { app, selectedComponent, componentControl, darkMode } =
		getContext<AppViewerContext>('AppViewerContext')

	let css = initCss($app.css?.aggridcomponent, customCss)

	// let result: any[] | undefined = undefined

	// $: result && setValues()

	// let value: any[] = Array.isArray(result)
	// 	? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() }))
	// 	: [{ error: 'input was not an array' }]

	// let loaded = false
	// async function setValues() {
	// 	value = Array.isArray(result)
	// 		? (result as any[]).map((x, i) => ({ ...x, __index: i.toString() }))
	// 		: [{ error: 'input was not an array' }]
	// 	if (api && loaded) {
	// 		let selected = api.getSelectedNodes()
	// 		if (selected && selected.length > 0 && resolvedConfig?.selectFirstRowByDefault != false) {
	// 			let data = { ...selected[0].data }
	// 			delete data['__index']
	// 			outputs?.selectedRow?.set(data)
	// 		}
	// 	}
	// 	if (!loaded) {
	// 		loaded = true
	// 	}
	// }

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

	let clientHeight
	let clientWidth

	const dispatch = createEventDispatcher()

	function onCellValueChanged(event) {
		let dataCell = event.newValue
		outputs?.newChange?.set({
			row: event.node.rowIndex,
			column: event.colDef.field,
			value: dataCell
		})
		// result[event.node.rowIndex][event.colDef.field] = dataCell
		// let data = { ...result[event.node.rowIndex] }
		// outputs?.selectedRow?.set(data)

		dispatch('update', {
			row: event.node.rowIndex,
			column: event.colDef.field,
			value: dataCell,
			data: event.node.data,
			oldValue: event.oldValue,
			columnDef: event.colDef
		})
	}

	let api: GridApi<any> | undefined = undefined
	let eGui: HTMLDivElement

	$: eGui && mountGrid()

	function transformColumnDefs(columnDefs: any[]) {
		const { isValid, errors } = validateColumnDefs(columnDefs)

		if (!isValid) {
			sendUserToast(`Invalid columnDefs: ${errors.join('\n')}`, true)
			return []
		}

		let r = columnDefs?.filter((x) => x && !x.ignored) ?? []
		if (allowDelete) {
			r.push({
				field: 'delete',
				headerName: 'Delete',
				cellRenderer: cellRendererFactory((c, p) => {
					new Button({
						target: c.eGui,
						props: {
							btnClasses: 'w-12',
							wrapperClasses: 'flex justify-end items-center h-full',
							color: 'light',
							size: 'sm',
							variant: 'contained',
							iconOnly: true,
							startIcon: { icon: Trash2 },
							nonCaptureEvent: true
						}
					})
				}),
				cellRendererParams: {
					onClick: (e) => {
						dispatch('delete', e)
					}
				},
				lockPosition: 'right',
				editable: false,
				flex: 0,
				width: 100
			})
		}
		return r
	}

	let firstRow = 0
	let lastRow = 0

	function validateColumnDefs(columnDefs: ColumnDef[]): { isValid: boolean; errors: string[] } {
		let isValid = true
		const errors: string[] = []

		// Validate each column definition
		columnDefs.forEach((colDef, index) => {
			// Check if 'field' property exists and is a non-empty string
			if (!colDef.field || typeof colDef.field !== 'string' || colDef.field.trim() === '') {
				isValid = false
				errors.push(`Column at index ${index} is missing a valid 'field' property.`)
			}
		})

		return { isValid, errors }
	}

	function mountGrid() {
		if (eGui) {
			createGrid(
				eGui,
				{
					rowModelType: 'infinite',
					datasource,
					columnDefs: transformColumnDefs(resolvedConfig?.columnDefs),
					pagination: false,
					defaultColDef: {
						flex: resolvedConfig.flex ? 1 : 0,
						editable: resolvedConfig?.allEditable,
						onCellValueChanged
					},
					infiniteInitialRowCount: 100,
					cacheBlockSize: 100,
					cacheOverflowSize: 10,
					maxBlocksInCache: 20,
					suppressColumnMoveAnimation: true,
					rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
					rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
						? resolvedConfig.rowMultiselectWithClick
						: false,
					initialState: state,
					suppressRowDeselection: true,
					...(resolvedConfig?.extraConfig ?? {}),
					onViewportChanged: (e) => {
						firstRow = e.firstRow
						lastRow = e.lastRow
					},
					onStateUpdated: (e) => {
						state = e?.api?.getState()
						resolvedConfig?.extraConfig?.['onStateUpdated']?.(e)
					},
					onGridReady: (e) => {
						outputs?.ready.set(true)

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
					getRowId: (data) => {
						return (data as any).data['__index']
					}
				},
				{}
			)
		}
	}

	$: resolvedConfig && updateOptions()

	let oldDatasource = datasource
	$: if (datasource && datasource != oldDatasource) {
		oldDatasource = datasource

		api?.updateGridOptions({ datasource })
	}

	let extraConfig = resolvedConfig.extraConfig
	$: if (!deepEqual(extraConfig, resolvedConfig.extraConfig)) {
		extraConfig = resolvedConfig.extraConfig
		if (extraConfig) {
			api?.updateGridOptions(extraConfig)
		}
	}

	export function clearRows() {
		api?.purgeInfiniteCache()
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

	function updateOptions() {
		api?.updateGridOptions({
			columnDefs: transformColumnDefs(resolvedConfig?.columnDefs),
			defaultColDef: {
				flex: resolvedConfig.flex ? 1 : 0,
				editable: resolvedConfig?.allEditable,
				onCellValueChanged
			},
			rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
			rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
				? resolvedConfig.rowMultiselectWithClick
				: false,
			...(resolvedConfig?.extraConfig ?? {})
		})
	}
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.tablecomponent}
	/>
{/each}

{#if Array.isArray(resolvedConfig.columnDefs) && resolvedConfig.columnDefs.every(isObject)}
	<div
		class={twMerge('divide-y flex flex-col h-full', css?.container?.class, 'wm-aggrid-container')}
		style={containerHeight ? `height: ${containerHeight}px;` : css?.container?.style}
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
			<div bind:this={eGui} style:height="100%" />
		</div>
		<div class="flex gap-1 w-full justify-end text-sm text-secondary py-1"
			>{firstRow}{'->'}{lastRow + 1} of {datasource?.rowCount} rows</div
		>
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
