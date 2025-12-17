<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import { MoreVertical, Plus, Table2, Trash2Icon, Pen } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput, Drawer, DrawerContent } from './common'
	import { sendUserToast } from '$lib/toast'
	import {
		type ColumnDef,
		type ForeignKeyMetadata,
		type TableMetadata
	} from './apps/components/display/dbtable/utils'
	import DBTable from './DBTable.svelte'
	import type { IDbSchemaOps, IDbTableOps } from './dbOps'
	import DropdownV2 from './DropdownV2.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Button from './common/button/Button.svelte'
	import DbTableEditor from './DBTableEditor.svelte'
	import type { DbType } from './dbTypes'
	import Portal from './Portal.svelte'
	import type { CreateTableValues } from './apps/components/display/dbtable/queries/createTable'
	import { diffTables } from './apps/components/display/dbtable/queries/diffTables'
	import { getContext, onMount } from 'svelte'
	import { buildCreateTableValues } from './apps/components/display/dbtable/queries/mergeMetaData'
	import { SvelteMap } from 'svelte/reactivity'

	type Props = {
		dbType: DbType
		dbSchema: DBSchema
		dbSupportsSchemas: boolean
		getColDefs: (tableKey: string) => Promise<ColumnDef[]>
		dbTableOpsFactory: (params: { colDefs: ColumnDef[]; tableKey: string }) => IDbTableOps
		dbSchemaOps: IDbSchemaOps
		refresh?: () => void
		initialTableKey?: string
	}

	let {
		dbType,
		dbSchema,
		dbTableOpsFactory,
		dbSchemaOps,
		getColDefs,
		dbSupportsSchemas,
		refresh,
		initialTableKey
	}: Props = $props()

	let schemaKeys = $derived(Object.keys(dbSchema.schema ?? {}))
	let search = $state('')
	let selected: {
		schemaKey?: undefined | string
		tableKey?: undefined | string
	} = $state({})

	$effect(() => {
		if (!selected.schemaKey && schemaKeys.length) {
			let schemaKey =
				'public' in dbSchema.schema ? 'public' : 'dbo' in dbSchema.schema ? 'dbo' : schemaKeys[0]
			let tableKey =
				initialTableKey && dbSchema.schema?.[schemaKey]?.[initialTableKey]
					? initialTableKey
					: undefined
			selected = { schemaKey, tableKey }
		}
	})

	let tableKeys = $derived.by(() => {
		if (dbSchema.lang === 'graphql') {
			sendUserToast('graphql not supported by DBExplorerTable', true)
			return []
		}
		if (!selected.schemaKey) return []
		return Object.keys(dbSchema.schema[selected.schemaKey] ?? {})
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
		dbSupportsSchemas && selected.schemaKey
			? `${selected.schemaKey}.${selected.tableKey}`
			: selected.tableKey
	)

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & { onConfirm: () => void })
		| undefined = $state()

	let dbTableEditorState: { open: boolean; action?: 'create' | 'update' } = $state({
		open: false,
		action: 'create'
	})

	const tableKeyToValuesMap = new SvelteMap<string, CreateTableValues>()
	let selectedValues: CreateTableValues | undefined = $state()

	const loadMetaData = getContext('loadAllTablesMetaData') as () => Promise<
		Record<string, TableMetadata> | undefined
	>

	const fetchForeignKeys = getContext('fetchForeignKeys') as (
		table: string
	) => Promise<ForeignKeyMetadata[] | undefined>

	let allTablesMetaData: Record<string, TableMetadata> | undefined

	// @ts-ignore
	$effect(async () => {
		dbTableEditorState.open

		if (selected.tableKey && tableKey) {
			const fks = await fetchForeignKeys(selected.tableKey)
			const tableMetaData = allTablesMetaData?.[tableKey]

			if (fks && tableMetaData) {
				selectedValues = buildCreateTableValues(selected.tableKey, tableMetaData, fks)
				tableKeyToValuesMap.set(selected.tableKey, selectedValues)
			}
		}
	})

	onMount(async () => {
		allTablesMetaData = await loadMetaData()
	})
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
					<Table2 class="text-primary shrink-0" size={16} />
					<p class="truncate text-ellipsis grow text-left text-emphasis text-xs">{tableKey}</p>
					<DropdownV2
						items={() => [
							{
								displayName: 'Edit table',
								icon: Pen,
								action: () => (dbTableEditorState = { open: true, action: 'update' })
							},
							{
								displayName: 'Delete table',
								icon: Trash2Icon,
								type: 'delete',
								action: () =>
									(askingForConfirmation = {
										title: `Are you sure you want to delete ${tableKey} ? This action is irreversible`,
										confirmationText: 'Delete permanently',
										open: true,
										onConfirm: async () => {
											askingForConfirmation && (askingForConfirmation.loading = true)
											try {
												await dbSchemaOps.onDelete({ tableKey, schema: selected.schemaKey })
												refresh?.()
												sendUserToast(`Table '${tableKey}' deleted successfully`)
											} catch (e) {
												let msg: string | undefined = (e as Error).message
												if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
												sendUserToast(msg ?? 'Action failed!', true)
											}
											askingForConfirmation = undefined
										}
									})
							}
						]}
						class="w-fit"
					>
						<svelte:fragment slot="buttonReplacement">
							<MoreVertical
								size={8}
								class="w-8 h-8 p-2 hover:bg-surface-hover cursor-pointer rounded-md"
							/>
						</svelte:fragment>
					</DropdownV2>
				</button>
			{/each}
		</div>
		<Button
			on:click={() => (dbTableEditorState = { open: true, action: 'create' })}
			wrapperClasses="mx-2 my-2 text-sm"
			startIcon={{ icon: Plus }}
			variant={tableKeys.length === 0 ? 'accent' : 'default'}
		>
			New table
		</Button>
	</Pane>
	<Pane class="p-3 pt-1">
		{#if tableKey}
			{#await getColDefs(tableKey) then colDefs}
				{#if colDefs && colDefs?.length}
					{@const dbTableOps = dbTableOpsFactory({ colDefs, tableKey })}
					<DBTable {dbTableOps} />
				{/if}
			{/await}
		{/if}
	</Pane>
</Splitpanes>

<Portal>
	<ConfirmationModal
		{...askingForConfirmation ?? { confirmationText: '', title: '' }}
		on:canceled={() => (askingForConfirmation = undefined)}
		on:confirmed={askingForConfirmation?.onConfirm ?? (() => {})}
	/>
</Portal>

<Drawer size="600px" open={dbTableEditorState.open}>
	<DrawerContent
		on:close={() => (dbTableEditorState = { open: false, action: undefined })}
		title={dbTableEditorState?.action === 'update' ? 'Edit table' : 'Create a new table'}
	>
		<DbTableEditor
			{dbSchema}
			currentSchema={selected.schemaKey}
			oldTableValues={dbTableEditorState.action === 'update' ? selectedValues : undefined}
			onConfirm={async (values) => {
				switch (dbTableEditorState.action) {
					case 'update':
						const oldValues = tableKeyToValuesMap.get(selected.tableKey ?? '')
						if (!oldValues) break

						const diffValues = diffTables(oldValues, values)
						if (!diffValues) break

						await dbSchemaOps.onAlter({ values: diffValues, schema: selected.schemaKey })
						tableKeyToValuesMap.set(selected.tableKey ?? '', $state.snapshot(values))
						break

					default:
						await dbSchemaOps.onCreate({ values, schema: selected.schemaKey })
						tableKeyToValuesMap.set(values.name ?? '', $state.snapshot(values))
						break
				}

				refresh?.()
				dbTableEditorState = { open: false }
			}}
			{dbType}
			previewSql={(values) => {
				if (dbTableEditorState.action === 'update') {
					const oldValues = tableKeyToValuesMap.get(selected.tableKey ?? '')
					if (!oldValues) return ''

					const diffValues = diffTables(oldValues, values)
					if (!diffValues) return ''
					return dbSchemaOps.previewAlterSql({ values: diffValues, schema: selected.schemaKey })
				}
				return dbSchemaOps.previewCreateSql({ values, schema: selected.schemaKey })
			}}
		/>
	</DrawerContent>
</Drawer>
