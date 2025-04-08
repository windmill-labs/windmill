<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import { Table2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput } from './common'
	import { sendUserToast } from '$lib/toast'
	import { type DbType } from './apps/components/display/dbtable/utils'
	import DBTable from './DBTable.svelte'

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
</script>

<Splitpanes>
	<Pane size={20}>
		<div class="mx-3 mt-3">
			<select
				value={selected.schemaKey}
				onchange={(e) => {
					selected = { schemaKey: e.currentTarget.value }
				}}
			>
				{#each schemaKeys as schemaKey}
					<option value={schemaKey}>{schemaKey}</option>
				{/each}
			</select>
			<ClearableInput wrapperClass="mt-3" bind:value={search} placeholder="Search table..." />
		</div>
		<div class="overflow-x-clip relative mt-3">
			{#each filteredTableKeys as tableKey}
				<button
					class={'w-full text-sm font-normal flex gap-2 items-center py-2 cursor-pointer px-3 ' +
						(selected.tableKey === tableKey ? 'bg-gray-500/25' : 'hover:bg-gray-500/10')}
					onclick={() => (selected.tableKey = tableKey)}
				>
					<Table2 class="text-gray-500/40 shrink-0" size={16} />
					<p class="truncate text-ellipsis">{tableKey}</p>
				</button>
			{/each}
		</div>
	</Pane>
	<Pane class="p-3 pt-1">
		<DBTable
			{resourcePath}
			{resourceType}
			tableKey={`${selected.schemaKey}.${selected.tableKey}`}
		/>
	</Pane>
</Splitpanes>
