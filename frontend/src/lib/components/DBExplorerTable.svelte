<script lang="ts">
	import { workspaceStore, type DBSchema } from '$lib/stores'
	import { Table2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput } from './common'
	import AppDbExplorer from './apps/components/display/dbtable/AppDbExplorer.svelte'
	import { sendUserToast } from '$lib/toast'
	import {
		loadTableMetaData,
		type DbType,
		type TableMetadata
	} from './apps/components/display/dbtable/utils'

	type Props = {
		dbSchema: DBSchema
		resourceType: DbType
		resourcePath: string
	}
	let { dbSchema, resourceType, resourcePath }: Props = $props()

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

	let tableKeys = $derived.by(() => {
		if (dbSchema.lang === 'graphql') {
			sendUserToast('graphql not supported by DBExplorerTable', true)
			return []
		}
		if (!selected.schemaKey) return []
		return Object.keys(dbSchema.schema[selected.schemaKey])
	})

	$effect(() => {
		if (tableKeys.length && !selected.tableKey) {
			selected.tableKey = filteredTableKeys[0]
		}
	})

	let filteredTableKeys = $derived.by(() => {
		const l = tableKeys.filter((tk) => tk.includes(search))
		l.sort()
		return l
	})

	let tableMetadata: TableMetadata | undefined = $state()
	$effect(() => {
		tableMetadata = undefined
		loadTableMetaData(
			'$res:' + resourcePath,
			$workspaceStore,
			selected.tableKey,
			resourceType
		).then((tm) => {
			tableMetadata = tm
		})
	})
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
	<Pane class="p-3 pt-1">
		{#if selected.tableKey && tableMetadata}
			{#key (selected.schemaKey, selected.tableKey)}
				<AppDbExplorer
					render
					id=""
					configuration={{
						type: {
							type: 'oneOf',
							selected: resourceType,
							configuration: {
								[resourceType]: {
									resource: {
										type: 'static',
										value: '$res:' + resourcePath
									},
									table: {
										type: 'static',
										selectOptions: tableKeys,
										loading: false,
										value: selected.tableKey
									}
								}
							}
						},
						columnDefs: {
							value: tableMetadata,
							type: 'static',
							loading: false
						},
						rowIdCol: { type: 'static', value: '' },
						whereClause: { type: 'static', value: '' },
						flex: { type: 'static', value: true },
						allEditable: { type: 'static', value: true },
						allowDelete: { type: 'static', value: true },
						multipleSelectable: { type: 'static', value: false },
						rowMultiselectWithClick: { type: 'static', value: true },
						selectFirstRowByDefault: { type: 'static', value: true },
						extraConfig: { type: 'static', value: {} },
						hideInsert: { type: 'static', value: false },
						hideSearch: { type: 'static', value: false },
						wrapActions: { type: 'static', value: false },
						footer: { type: 'static', value: true },
						customActionsHeader: { type: 'static' }
					}}
				/>
			{/key}
		{/if}
	</Pane>
</Splitpanes>
