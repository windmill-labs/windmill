<script lang="ts">
	import { GridApi, createGrid, type IDatasource } from 'ag-grid-community'
	import { sendUserToast } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext, ComponentCustomCSS, ContextPanelContext } from '../../../types'

	import type { TableAction, components } from '$lib/components/apps/editor/component'
	import { deepEqual } from 'fast-equals'
	import SyncColumnDefs from './SyncColumnDefs.svelte'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '$lib/components/apps/utils'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'
	import type { Output } from '$lib/components/apps/rx'
	import type { InitConfig } from '$lib/components/apps/editor/appUtils'
	import { Button } from '$lib/components/common'
	import { cellRendererFactory, defaultCellRenderer } from './utils'
	import { Download, Trash2 } from 'lucide-svelte'
	import type { ColumnDef } from '../dbtable/utils'
	import AppAggridTableActions from './AppAggridTableActions.svelte'
	import Popover from '$lib/components/Popover.svelte'

	export let id: string
	export let customCss: ComponentCustomCSS<'aggridcomponent'> | undefined = undefined
	export let containerHeight: number | undefined = undefined
	export let resolvedConfig: InitConfig<
		| (typeof components)['dbexplorercomponent']['initialData']['configuration']
		| (typeof components)['aggridinfinitecomponent']['initialData']['configuration']
		| (typeof components)['aggridinfinitecomponentee']['initialData']['configuration']
	>
	export let datasource: IDatasource
	export let state: any = undefined
	export let outputs: Record<string, Output<any>>
	export let allowDelete: boolean
	export let actions: TableAction[] = []
	export let result: any[] | undefined = undefined
	export let allowColumnDefsActions: boolean = true
	let inputs = {}

	const context = getContext<AppViewerContext>('AppViewerContext')
	const contextPanel = getContext<ContextPanelContext>('ContextPanel')
	const { app, selectedComponent, componentControl, darkMode } = context

	let css = initCss($app.css?.aggridcomponent, customCss)

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

	function refreshActions(actions: TableAction[]) {
		if (!deepEqual(actions, lastActions)) {
			lastActions = [...actions]

			updateOptions()
		}
	}

	let lastActions: TableAction[] | undefined = undefined
	$: actions && refreshActions(actions)

	const tableActionsFactory = cellRendererFactory((c, p) => {
		const rowIndex = p.node.rowIndex ?? 0
		const row = p.data

		const componentContext = new Map<string, any>([
			['AppViewerContext', context],
			['ContextPanel', contextPanel]
		])

		new AppAggridTableActions({
			target: c.eGui,
			props: {
				id: id,
				actions,
				rowIndex,
				row,
				render: true,
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
			context: componentContext
		})
	})

	function transformColumnDefs(columnDefs: any[] | undefined) {
		if (!columnDefs) {
			return []
		}

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

		if (actions && actions.length > 0) {
			r.push({
				headerName: resolvedConfig?.customActionsHeader
					? resolvedConfig?.customActionsHeader
					: 'Actions',
				cellRenderer: tableActionsFactory,
				autoHeight: true,
				cellStyle: { textAlign: 'center' },
				cellClass: 'grid-cell-centered',
				lockPosition: 'right',

				...(!resolvedConfig?.wrapActions ? { minWidth: 130 * actions?.length } : {})
			})
		}

		return r.map((fields) => {
			let cr = defaultCellRenderer(fields.cellRendererType)
			return {
				...fields,
				...(cr ? { cellRenderer: cr } : {})
			}
		})
	}

	let firstRow: number = 0
	let lastRow: number = 0

	function validateColumnDefs(columnDefs: ColumnDef[]): {
		isValid: boolean
		errors: string[]
	} {
		let isValid = true
		const errors: string[] = []

		if (!Array.isArray(columnDefs)) {
			return { isValid: false, errors: ['Column definitions must be an array.'] }
		}

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
					...(resolvedConfig?.wrapActions
						? {
								rowHeight: Math.max(44, actions.length * 48)
						  }
						: {
								rowHeight: 44
						  }),
					suppressColumnMoveAnimation: true,
					suppressDragLeaveHidesColumns: true,
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
							},
							recompute: () => {
								dispatch('recompute')
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

	$: api && resolvedConfig && updateOptions()

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

	export function restoreColumns() {
		api?.resetColumnState()
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
		// console.debug('updateOptions', resolvedConfig, api)
		api?.updateGridOptions({
			columnDefs: transformColumnDefs(resolvedConfig?.columnDefs),
			defaultColDef: {
				flex: resolvedConfig.flex ? 1 : 0,
				editable: resolvedConfig?.allEditable,
				onCellValueChanged
			},
			suppressDragLeaveHidesColumns: true,
			...(resolvedConfig?.wrapActions
				? {
						rowHeight: Math.max(44, actions.length * 48)
				  }
				: {
						rowHeight: 44
				  }),
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

<SyncColumnDefs {id} columnDefs={resolvedConfig.columnDefs} {result} {allowColumnDefsActions}>
	<div
		class={twMerge(
			'flex flex-col h-full component-wrapper divide-y',
			css?.container?.class,
			'wm-aggrid-container'
		)}
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
		{#if resolvedConfig && 'footer' in resolvedConfig && resolvedConfig.footer}
			<div class="flex gap-1 w-full justify-between items-center text-xs text-primary p-2">
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
				{#if datasource?.rowCount}
					{firstRow}{'->'}{lastRow + 1} of {datasource?.rowCount} rows
				{:else}
					{firstRow}{'->'}{lastRow + 1}
				{/if}
			</div>
		{/if}
	</div>
</SyncColumnDefs>
