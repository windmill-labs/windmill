<script lang="ts">
	import type { DBSchema } from '$lib/stores'
	import { type IGetRowsParams } from 'ag-grid-community'
	import { createGrid } from 'ag-grid-community'
	import { Table2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput } from './common'

	type Props = {
		dbSchema: DBSchema
	}
	let { dbSchema }: Props = $props()

	let schemaKeys = $derived(Object.keys(dbSchema.schema))
	let search = $state('')
	let selected: {
		schemaKey?: undefined | string
		tableKey?: undefined | string
	} = $state({})
	$effect(() => {
		if (!selected.schemaKey && schemaKeys.length) {
			selected = { schemaKey: schemaKeys[0] }
		}
	})

	let tableKeys = $derived(
		selected.schemaKey ? Object.keys(dbSchema.schema[selected.schemaKey]) : []
	)

	let filteredTableKeys = $derived(tableKeys.filter((tk) => tk.includes(search)))

	let eGui: HTMLDivElement | undefined = $state()

	$effect(() => eGui && mountGrid())

	const datasource = {
		getRows: async (params: IGetRowsParams) => {
			params.successCallback([])
		}
	}

	function mountGrid() {
		if (eGui) {
			createGrid(
				eGui,
				{
					rowModelType: 'infinite',
					datasource,
					columnDefs: [],
					pagination: false,
					defaultColDef: {
						flex: 1,
						editable: true,
						onCellValueChanged: (e) => {
							console.log(e)
						}
					},
					infiniteInitialRowCount: 100,
					cacheBlockSize: 100,
					cacheOverflowSize: 10,
					maxBlocksInCache: 20,
					rowHeight: 44,
					suppressColumnMoveAnimation: true,
					suppressDragLeaveHidesColumns: true,
					rowSelection: 'single',
					suppressRowDeselection: true,
					enableCellTextSelection: true
				},
				{}
			)
		}
	}
</script>

<Splitpanes>
	<Pane size={20}>
		<div class="mx-3 mt-3">
			<select
				value={selected.schemaKey}
				onchange={(e) => {
					selected = { schemaKey: e.currentTarget.value }
					console.log(e.currentTarget.value)
				}}
			>
				{#each schemaKeys as schemaKey}
					<option value={schemaKey}>{schemaKey}</option>
				{/each}
			</select>
			<ClearableInput
				wrapperClass="mt-3"
				bind:value={search}
				placeholder="Search outputs..."
			/></div
		>
		<div class="overflow-x-clip relative mt-3">
			{#each filteredTableKeys as tableKey}
				<button
					class={'w-full text-sm font-normal flex gap-2 items-center py-2 cursor-pointer px-3 ' +
						(selected.tableKey === tableKey ? 'bg-gray-200' : 'hover:bg-gray-100')}
					onclick={() => (selected.tableKey = tableKey)}
				>
					<Table2 class="text-gray-400 shrink-0" size={16} />
					<p class="truncate text-ellipsis">{tableKey}</p>
				</button>
			{/each}
		</div>
	</Pane>
	<Pane></Pane>
</Splitpanes>

<!-- <div bind:this={eGui} style:height="100%"></div> -->
