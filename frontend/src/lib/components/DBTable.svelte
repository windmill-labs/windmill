<script lang="ts" module>
	import type { ColDef, ITooltipComp, ITooltipParams } from 'ag-grid-community'
	import type { TableEditorForeignKey } from './apps/components/display/dbtable/tableEditor'
	import { mount, unmount } from 'svelte'
	import DbForeignKeyTooltip from './DbForeignKeyTooltip.svelte'

	export type DbRowFilter = { column: string; value: unknown }
	export type DbForeignKeyTarget = { table: string; column: string; value: unknown }

	type FkTooltipValue = { table: string; column: string; value: unknown }

	/** AG Grid tooltip hosting the "Go to row" popover of a foreign-keyed cell. */
	class ForeignKeyTooltip implements ITooltipComp {
		private eGui = document.createElement('div')
		private component: ReturnType<typeof mount> | undefined

		init(params: ITooltipParams & { onGoToRow: (target: DbForeignKeyTarget) => void }) {
			const target = params.value as FkTooltipValue
			this.component = mount(DbForeignKeyTooltip, {
				target: this.eGui,
				props: {
					targetTable: target.table,
					targetColumn: target.column,
					onGoToRow: () => {
						params.hideTooltipCallback?.()
						params.onGoToRow(target)
					}
				}
			})
		}
		getGui() {
			return this.eGui
		}
		destroy() {
			if (this.component) unmount(this.component)
		}
	}

	/** Single-column foreign keys only: a composite key has no one cell value to follow. */
	function foreignKeyColDefs(
		foreignKeys: TableEditorForeignKey[],
		onGoToRow: (target: DbForeignKeyTarget) => void
	): Record<string, Partial<ColDef>> {
		const out: Record<string, Partial<ColDef>> = {}
		for (const fk of foreignKeys) {
			if (fk.columns.length !== 1) continue
			const { sourceColumn, targetColumn } = fk.columns[0]
			if (!sourceColumn || !targetColumn || !fk.targetTable) continue
			const table = fk.targetTable
			out[sourceColumn] = {
				tooltipComponent: ForeignKeyTooltip,
				tooltipComponentParams: { onGoToRow },
				tooltipValueGetter: (p): FkTooltipValue | null =>
					p.value === null || p.value === undefined || p.value === ''
						? null
						: { table, column: targetColumn, value: p.value },
				cellClass: 'underline decoration-dotted decoration-tertiary underline-offset-2'
			}
		}
		return out
	}
</script>

<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createGrid, type GridApi, type IDatasource } from 'ag-grid-community'
	import { transformColumnDefs } from './apps/components/display/table/utils'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Button } from './common'
	import { Download, X } from 'lucide-svelte'
	import Popover from './Popover.svelte'
	import DebouncedInput from './apps/components/helpers/DebouncedInput.svelte'
	import InsertRowDrawerButton from './apps/components/display/InsertRowDrawerButton.svelte'
	import type { IDbTableOps } from './dbOps'
	import { deepEqual } from 'fast-equals'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'
	import '$lib/components/apps/components/display/table/theme/windmill-theme.css'
	import { untrack } from 'svelte'

	type Props = {
		dbTableOps: IDbTableOps
		/** Foreign keys of the displayed table; their cells get a "Go to row" popover. */
		foreignKeys?: TableEditorForeignKey[]
		onGoToRow?: (target: DbForeignKeyTarget) => void
		/** Equality filter already applied by `dbTableOps`, shown as a dismissable chip. */
		rowFilter?: DbRowFilter
		onClearRowFilter?: () => void
	}
	let { dbTableOps, foreignKeys, onGoToRow, rowFilter, onClearRowFilter }: Props = $props()

	let [clientHeight, clientWidth, darkMode, firstRow, lastRow] = $state([0, 0, false, -1, -1])
	let quicksearch = $state('')
	let api: GridApi<any> | undefined = $state()
	let eGui: HTMLDivElement | undefined = $state()

	let datasource: IDatasource = {
		getRows: async function (params) {
			if (!$workspaceStore) return params.failCallback()
			let lastRow = rowCount && rowCount <= params.endRow ? rowCount : -1

			const items = await dbTableOps.getRows({
				offset: params.startRow,
				limit: params.endRow - params.startRow,
				quicksearch: params.context.quicksearch,
				order_by: params.sortModel?.[0]?.colId ?? dbTableOps.colDefs[0]?.field,
				is_desc: params.sortModel?.[0]?.sort === 'desc'
			})

			if (items.length < params.endRow - params.startRow) lastRow = params.startRow + items.length
			params.successCallback(items, lastRow)
		}
	}
	let rowCount = $state(0)
	let refreshCount = $state(0)
	export const refresh = () => (refreshCount += 1)

	$effect(() => eGui && untrack(() => mountGrid()))
	function mountGrid() {
		if (eGui && !api) {
			createGrid(eGui, {
				rowModelType: 'infinite',
				pagination: false,
				...(dbTableOps.onUpdate && {
					defaultColDef: {
						flex: 1,
						minWidth: 150,
						editable: true,
						onCellValueChanged: (e) => {
							if (!$workspaceStore) return
							const colDef = e.colDef as unknown as { field: string; datatype: string }
							dbTableOps
								.onUpdate?.(
									{ values: { ...(e.data as object), [colDef.field]: e.oldValue } },
									colDef,
									e.newValue
								)
								.then(() => {
									sendUserToast('Value updated')
								})
								.catch((e) => {
									sendUserToast('Error updating value: ' + ((e as Error)?.message || e), true)
									refresh?.()
								})
						}
					}
				}),
				onViewportChanged: (e) => ([firstRow, lastRow] = [e.firstRow, e.lastRow]),
				// Lets the pointer move from a foreign-keyed cell into its "Go to row" popover.
				tooltipInteraction: true,
				// AG Grid floors the show delay at 200ms whatever value is given.
				tooltipShowDelay: 0,
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

	let prevUpdateKey: any = undefined
	$effect(() => {
		if (!$workspaceStore || !api) return
		const key = { quicksearch, colDefs: dbTableOps.colDefs, refreshCount, rowFilter }
		if (deepEqual(key, prevUpdateKey)) return
		prevUpdateKey = key
		untrack(() => updateGrid())
	})
	// Foreign keys arrive after the rows: refresh the column defs alone so the
	// popovers appear without re-running the row and count queries.
	let prevForeignKeys: TableEditorForeignKey[] | undefined = undefined
	$effect(() => {
		if (!api || foreignKeys === prevForeignKeys) return
		prevForeignKeys = foreignKeys
		untrack(() => api?.updateGridOptions({ columnDefs: buildColumnDefs() }))
	})
	function buildColumnDefs() {
		const fkColDefs = foreignKeys && onGoToRow ? foreignKeyColDefs(foreignKeys, onGoToRow) : {}
		return transformColumnDefs({
			columnDefs: (dbTableOps.colDefs ?? []).map((c) =>
				c.field && fkColDefs[c.field] ? { ...c, ...fkColDefs[c.field] } : c
			),
			...(dbTableOps.onDelete && {
				onDelete: (values) => {
					if (!$workspaceStore) return
					dbTableOps
						.onDelete?.({ values })
						.then(() => {
							refresh?.()
							sendUserToast('Row deleted')
						})
						.catch((e) => {
							sendUserToast(`Error deleting row: ${e?.message ?? e}`, true)
						})
				}
			})
		})
	}
	function updateGrid() {
		dbTableOps.getCount({ quicksearch }).then((result) => (rowCount = result))

		api?.purgeInfiniteCache()
		api?.updateGridOptions({
			datasource,
			columnDefs: buildColumnDefs(),
			context: {
				quicksearch
			}
		})
	}
</script>

<DarkModeObserver bind:darkMode />

<div class="h-full relative flex flex-col">
	<div class="flex py-2 h-12 justify-between gap-4">
		<div class="flex w-full items-center gap-2">
			<DebouncedInput
				class="w-full max-w-[300px]"
				type="text"
				bind:value={quicksearch}
				placeholder="Search..."
			/>
			{#if rowFilter}
				<div
					class="flex h-7 items-center gap-1 whitespace-nowrap rounded-md border bg-surface-secondary pl-2 pr-0.5 text-xs text-primary"
					data-testid="db-row-filter-chip"
				>
					<span class="font-mono">{rowFilter.column} = {String(rowFilter.value)}</span>
					<Button
						iconOnly
						unifiedSize="2xs"
						variant="subtle"
						startIcon={{ icon: X }}
						title="Clear filter"
						onClick={onClearRowFilter}
					/>
				</div>
			{/if}
		</div>
		{#if dbTableOps.onInsert}
			<InsertRowDrawerButton
				columnDefs={dbTableOps.colDefs ?? []}
				dbType={dbTableOps.dbType}
				onInsert={(values) => {
					if (!$workspaceStore) return
					dbTableOps.onInsert?.({ values }).then((result) => {
						refresh?.()
						sendUserToast('Row inserted')
					})
				}}
			/>
		{/if}
	</div>
	<div
		class={'flex flex-col flex-1 component-wrapper divide-y wm-aggrid-container'}
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
				class={api ? '' : 'opacity-0'}
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
						on:click={() => api?.exportDataAsCsv()}
						iconOnly
					/>
				</Popover>
			</div>
			{#if rowCount}
				{firstRow}{'->'}{lastRow + 1} of {rowCount} rows
			{:else}
				{firstRow}{'->'}{lastRow + 1}
			{/if}
		</div>
	</div>
</div>
