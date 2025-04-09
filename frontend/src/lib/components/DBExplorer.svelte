<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import { MoreVertical, Table2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput } from './common'
	import { sendUserToast } from '$lib/toast'
	import { type ColumnDef } from './apps/components/display/dbtable/utils'
	import DBTable from './DBTable.svelte'
	import type { DbTableActionFactory, IDbTableOps } from './dbOps'
	import DropdownV2 from './DropdownV2.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'

	type Props = {
		dbSchema: DBSchema
		getColDefs: (tableKey: string) => Promise<ColumnDef[]>
		dbTableOpsFactory: (params: { colDefs: ColumnDef[]; tableKey: string }) => IDbTableOps
		dbTableActionsFactory?: DbTableActionFactory[]
	}
	let { dbSchema, dbTableOpsFactory, getColDefs, dbTableActionsFactory }: Props = $props()

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

	let tableKey = $derived(`${selected.schemaKey}.${selected.tableKey}`)

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & { onConfirm: () => void })
		| undefined = $state()
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
					class={'w-full text-sm font-normal flex gap-2 items-center h-10 cursor-pointer pl-3 pr-1 ' +
						(selected.tableKey === tableKey ? 'bg-gray-500/25' : 'hover:bg-gray-500/10')}
					onclick={() => (selected.tableKey = tableKey)}
				>
					<Table2 class="text-gray-500/40 shrink-0" size={16} />
					<p class="truncate text-ellipsis grow text-left">{tableKey}</p>
					{#if dbTableActionsFactory}
						{@const dbTableActions = dbTableActionsFactory.map((f) => f({ tableKey }))}
						<DropdownV2
							items={dbTableActions.map((tableAction) => ({
								displayName: tableAction.displayName,
								...(tableAction.icon ? { icon: tableAction.icon } : {}),
								action: () =>
									(askingForConfirmation = {
										title: tableAction.confirmTitle ?? 'Are you sure ?',
										confirmationText: tableAction.confirmBtnText ?? 'Confirm',
										open: true,
										onConfirm: async () => {
											askingForConfirmation && (askingForConfirmation.loading = true)
											await tableAction.action()
											askingForConfirmation = undefined
										}
									})
							}))}
							class="w-fit"
						>
							<svelte:fragment slot="buttonReplacement">
								<MoreVertical
									size={8}
									class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
								/>
							</svelte:fragment>
						</DropdownV2>
					{/if}
				</button>
			{/each}
		</div>
	</Pane>
	<Pane class="p-3 pt-1">
		{#await getColDefs(tableKey) then colDefs}
			{#if colDefs?.length}
				{@const dbTableOps = dbTableOpsFactory({ colDefs, tableKey })}
				<DBTable {dbTableOps} />
			{/if}
		{/await}
	</Pane>
</Splitpanes>

<ConfirmationModal
	{...askingForConfirmation ?? { confirmationText: '', title: '' }}
	on:canceled={() => (askingForConfirmation = undefined)}
	on:confirmed={askingForConfirmation?.onConfirm ?? (() => {})}
/>
