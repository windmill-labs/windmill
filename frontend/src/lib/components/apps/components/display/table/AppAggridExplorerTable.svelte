<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { type GridApi, createGrid, type IDatasource } from 'ag-grid-community'
	import { sendUserToast } from '$lib/utils'
	import { createEventDispatcher, getContext, mount, unmount, untrack } from 'svelte'
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
	import { ColumnIdentity, type ColumnDef } from '../dbtable/utils'
	import AppAggridTableActions from './AppAggridTableActions.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { stateSnapshot, withProps } from '$lib/svelte5Utils.svelte'
	import { get } from 'svelte/store'

	interface Props {
		id: string
		customCss?: ComponentCustomCSS<'aggridcomponent'> | undefined
		containerHeight?: number | undefined
		resolvedConfig: InitConfig<
			| (typeof components)['dbexplorercomponent']['initialData']['configuration']
			| (typeof components)['aggridinfinitecomponent']['initialData']['configuration']
			| (typeof components)['aggridinfinitecomponentee']['initialData']['configuration']
		>
		datasource: IDatasource
		componentState?: any
		outputs: Record<string, Output<any>>
		allowDelete: boolean
		actions?: TableAction[]
		result?: any[] | undefined
		allowColumnDefsActions?: boolean
		onChange?: string[] | undefined
	}

	let {
		id,
		customCss = undefined,
		containerHeight = undefined,
		resolvedConfig,
		datasource,
		componentState = $bindable(undefined),
		outputs,
		allowDelete,
		actions = [],
		result = undefined,
		allowColumnDefsActions = true,
		onChange = undefined
	}: Props = $props()
	let inputs = {}

	const context = getContext<AppViewerContext>('AppViewerContext')
	const contextPanel = getContext<ContextPanelContext>('ContextPanel')
	const { app, selectedComponent, componentControl, darkMode, mode } = context

	let css = $state(initCss($app.css?.aggridcomponent, customCss))

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

	let clientHeight = $state()
	let clientWidth = $state()

	const dispatch = createEventDispatcher()

	function fireOnChange() {
		let runnableComponents = get(context.runnableComponents)
		if (onChange) {
			onChange.forEach((id) => runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

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

		resolvedConfig?.extraConfig?.['defaultColDef']?.['onCellValueChanged']?.(event)
		fireOnChange()
	}

	let api: GridApi<any> | undefined = $state(undefined)
	let eGui: HTMLDivElement | undefined = $state()

	function refreshActions(actions: TableAction[]) {
		if (!deepEqual(actions, lastActions)) {
			lastActions = structuredClone(stateSnapshot(actions))
			updateOptions()
		}
	}

	let lastActions: TableAction[] | undefined = undefined

	const tableActionsFactory = cellRendererFactory((c, p) => {
		const rowIndex = p.node.rowIndex ?? 0
		const row = p.data

		const componentContext = new Map<string, any>([
			['AppViewerContext', context],
			['ContextPanel', contextPanel]
		])

		const taComponent = withProps(AppAggridTableActions, {
			p,
			id: id,
			actions,
			rowIndex,
			row,
			render: true,
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
		const ta = mount(taComponent.component, {
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
			}
		}
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
					const btnComponent = mount(Button, {
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
					return {
						destroy: () => {
							unmount(btnComponent)
						},
						refresh(params) {
							//
						}
					}
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

	let firstRow: number = $state(0)
	let lastRow: number = $state(0)

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
			let noField = !colDef.field || typeof colDef.field !== 'string' || colDef.field.trim() === ''

			if (
				(colDef.isidentity === ColumnIdentity.ByDefault ||
					colDef.isidentity === ColumnIdentity.Always) &&
				colDef.hideInsert == undefined
			) {
				colDef.hideInsert = true
			}

			// Check if 'field' property exists and is a non-empty string
			if (noField && !(colDef.children && Array.isArray(colDef.children))) {
				isValid = false
				errors.push(
					`Column at index ${index} is missing a valid 'field' property nor having any children.`
				)
			}

			if (colDef.children && Array.isArray(colDef.children)) {
				const { isValid: isChildrenValid, errors: childrenErrors } = validateColumnDefs(
					colDef.children
				)
				if (!isChildrenValid) {
					isValid = false
					errors.push(...childrenErrors.map((err) => `Error in children at index ${index}: ${err}`))
				}
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
					infiniteInitialRowCount: 100,
					cacheBlockSize: 100,
					cacheOverflowSize: 10,
					maxBlocksInCache: 20,
					...(resolvedConfig?.wrapActions
						? { rowHeight: Math.max(44, actions.length * 48) }
						: { rowHeight: 44 }),
					suppressColumnMoveAnimation: true,
					suppressDragLeaveHidesColumns: true,
					rowSelection: resolvedConfig?.multipleSelectable ? 'multiple' : 'single',
					rowMultiSelectWithClick: resolvedConfig?.multipleSelectable
						? resolvedConfig.rowMultiselectWithClick
						: false,
					initialState: componentState,
					suppressRowDeselection: true,
					enableCellTextSelection: true,
					...(resolvedConfig?.extraConfig ?? {}),
					defaultColDef: {
						flex: resolvedConfig.flex ? 1 : 0,
						editable: resolvedConfig?.allEditable,
						onCellValueChanged,
						...resolvedConfig?.extraConfig?.['defaultColDef']
					},
					onViewportChanged: (e) => {
						firstRow = e.firstRow
						lastRow = e.lastRow
					},
					onStateUpdated: (e) => {
						componentState = e?.api?.getState()
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
					getRowId: (data) =>
						resolvedConfig?.rowIdCol && resolvedConfig?.rowIdCol != ''
							? data.data?.[resolvedConfig?.rowIdCol]
							: (data.data?.['id'] ?? (data as any).data['__index'])
				},
				{}
			)
		}
	}

	let oldDatasource = $state(datasource)

	let extraConfig = $state(resolvedConfig.extraConfig)

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
			...(resolvedConfig?.extraConfig ?? {}),
			defaultColDef: {
				flex: resolvedConfig.flex ? 1 : 0,
				editable: resolvedConfig?.allEditable,
				onCellValueChanged,
				...resolvedConfig?.extraConfig?.['defaultColDef']
			}
		})
	}
	$effect(() => {
		eGui && untrack(() => mountGrid())
	})
	$effect(() => {
		actions && untrack(() => refreshActions(actions))
	})
	$effect(() => {
		api && resolvedConfig && updateOptions()
	})
	$effect(() => {
		if (api && datasource && datasource != oldDatasource) {
			oldDatasource = datasource
			untrack(() => {
				api?.updateGridOptions({ datasource })
			})
		}
	})
	$effect(() => {
		if (!deepEqual(extraConfig, resolvedConfig.extraConfig)) {
			extraConfig = resolvedConfig.extraConfig
			if (extraConfig && api) {
				untrack(() => {
					extraConfig && api?.updateGridOptions(extraConfig)
				})
			}
		}
	})
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
			onpointerdown={stopPropagation(() => {
				$selectedComponent = [id]
			})}
			style:height="{clientHeight}px"
			style:width="{clientWidth}px"
			class="ag-theme-alpine"
			class:ag-theme-alpine-dark={$darkMode}
		>
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
		</div>
		{#if resolvedConfig && 'footer' in resolvedConfig && resolvedConfig.footer}
			<div class="flex gap-1 w-full justify-between items-center text-xs text-primary p-2">
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
				{#if datasource?.rowCount}
					{firstRow}{'->'}{lastRow + 1} of {datasource?.rowCount} rows
				{:else}
					{firstRow}{'->'}{lastRow + 1}
				{/if}
			</div>
		{/if}
	</div>
</SyncColumnDefs>
