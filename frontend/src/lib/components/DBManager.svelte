<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import { MoreVertical, Table2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput, Drawer, DrawerContent } from './common'
	import { sendUserToast } from '$lib/toast'
	import { type ColumnDef } from './apps/components/display/dbtable/utils'
	import DBTable from './DBTable.svelte'
	import type { DbTableActionFactory, IDbTableOps } from './dbOps'
	import DropdownV2 from './DropdownV2.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Button from './common/button/Button.svelte'
	import DbTableEditor, { type DBTableEditorProps } from './DBTableEditor.svelte'

	type Props = {
		dbSchema: DBSchema
		dbSupportsSchemas: boolean
		getColDefs: (tableKey: string) => Promise<ColumnDef[]>
		dbTableOpsFactory: (params: { colDefs: ColumnDef[]; tableKey: string }) => IDbTableOps
		dbTableActionsFactory?: DbTableActionFactory[]
		refresh?: () => void
		refreshCount?: number
		dbTableEditorPropsFactory?: (params: { selectedSchemaKey?: string }) => DBTableEditorProps
	}
	let {
		dbSchema,
		dbTableOpsFactory,
		getColDefs,
		dbTableActionsFactory,
		refresh,
		dbTableEditorPropsFactory,
		dbSupportsSchemas,
		refreshCount
	}: Props = $props()

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

	let tableKey = $derived(
		dbSupportsSchemas ? `${selected.schemaKey}.${selected.tableKey}` : selected.tableKey
	)

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & { onConfirm: () => void })
		| undefined = $state()

	let dbTableEditorState: { open: boolean } = $state({ open: false })

	let dbTableEditorProps = $derived(
		dbTableEditorPropsFactory?.({ selectedSchemaKey: selected.schemaKey })
	)
</script>

<Splitpanes>
	<Pane size={24} class="relative flex flex-col">
		<div class="mx-3 mt-3">
			{#if dbSupportsSchemas}
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
			{/if}
			<ClearableInput wrapperClass="mt-3" bind:value={search} placeholder="Search table..." />
		</div>
		<div class="overflow-x-clip overflow-y-auto relative mt-3 border-y flex-1">
			{#each filteredTableKeys as tableKey}
				<button
					class={'w-full text-sm font-normal flex gap-2 items-center h-10 cursor-pointer pl-3 pr-1 ' +
						(selected.tableKey === tableKey ? 'bg-gray-500/25' : 'hover:bg-gray-500/10')}
					onclick={() => (selected.tableKey = tableKey)}
				>
					<Table2 class="text-gray-500/40 shrink-0" size={16} />
					<p class="truncate text-ellipsis grow text-left">{tableKey}</p>
					{#if dbTableActionsFactory}
						{@const dbTableActions = dbTableActionsFactory.map((f) =>
							f({
								tableKey: `${selected.schemaKey}.${tableKey}`,
								refresh: refresh ?? (() => {})
							})
						)}
						<DropdownV2
							items={() =>
								dbTableActions.map((tableAction) => ({
									displayName: tableAction.displayName,
									...(tableAction.icon ? { icon: tableAction.icon } : {}),
									action: () =>
										(askingForConfirmation = {
											title: tableAction.confirmTitle ?? 'Are you sure ?',
											confirmationText: tableAction.confirmBtnText ?? 'Confirm',
											open: true,
											onConfirm: async () => {
												askingForConfirmation && (askingForConfirmation.loading = true)
												try {
													await tableAction.action()
													tableAction.successText && sendUserToast(tableAction.successText)
												} catch (e) {
													let msg: string | undefined = (e as Error).message
													if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
													sendUserToast(msg ?? 'Action failed!', true)
												}
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
		<Button
			color="light"
			on:click={() => (dbTableEditorState = { open: true })}
			wrapperClasses="mx-2 my-1"
		>
			New table
		</Button>
	</Pane>
	<Pane class="p-3 pt-1">
		{#key tableKey}
			{#if tableKey}
				{#await getColDefs(tableKey) then colDefs}
					{#if colDefs?.length}
						{@const dbTableOps = dbTableOpsFactory({ colDefs, tableKey })}
						<DBTable {dbTableOps} {refresh} {refreshCount} />
					{/if}
				{/await}
			{/if}
		{/key}
	</Pane>
</Splitpanes>

<ConfirmationModal
	{...askingForConfirmation ?? { confirmationText: '', title: '' }}
	on:canceled={() => (askingForConfirmation = undefined)}
	on:confirmed={askingForConfirmation?.onConfirm ?? (() => {})}
/>

{#if dbTableEditorProps}
	<Drawer
		size="600px"
		open={dbTableEditorState.open}
		on:close={() => (dbTableEditorState = { open: false })}
	>
		<DrawerContent
			on:close={() => (dbTableEditorState = { open: false })}
			title="Create a new table"
		>
			<DbTableEditor
				{...dbTableEditorProps}
				{dbSchema}
				currentSchema={selected.schemaKey}
				onConfirm={async (values) => {
					await dbTableEditorProps.onConfirm(values)
					dbTableEditorState = { open: false }
				}}
			/>
		</DrawerContent>
	</Drawer>
{/if}
