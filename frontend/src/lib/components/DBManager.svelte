<script lang="ts">
	import { type DBSchema } from '$lib/stores'
	import {
		ChevronDownIcon,
		EditIcon,
		Loader2,
		MoreVertical,
		Plus,
		Table2,
		Trash2Icon
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput, Drawer, DrawerContent } from './common'
	import { sendUserToast } from '$lib/toast'
	import { type ColumnDef } from './apps/components/display/dbtable/utils'
	import DBTable from './DBTable.svelte'
	import type { IDbSchemaOps, IDbTableOps } from './dbOps'
	import DropdownV2 from './DropdownV2.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Button from './common/button/Button.svelte'
	import DbTableEditor from './DBTableEditor.svelte'
	import type { DbType } from './dbTypes'
	import Portal from './Portal.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import type { Snippet } from 'svelte'
	import { diffCreateTableValues } from './apps/components/display/dbtable/queries/alterTable'
	import { resource } from 'runed'

	/** Represents a selected table with its schema */
	export interface SelectedTable {
		schema: string
		table: string
	}

	type Props = {
		dbType: DbType
		dbSchema: DBSchema
		dbSupportsSchemas: boolean
		getColDefs: (tableKey: string) => Promise<ColumnDef[]>
		dbTableOpsFactory: (params: { colDefs: ColumnDef[]; tableKey: string }) => IDbTableOps
		dbSchemaOps: IDbSchemaOps
		refresh?: () => void
		initialSchemaKey?: string
		initialTableKey?: string
		selectedSchemaKey?: string | undefined
		selectedTableKey?: string | undefined
		dbSelector?: Snippet<[]>
		/** Enable multi-select mode with checkboxes in sidebar */
		multiSelectMode?: boolean
		/** Selected tables in multi-select mode */
		selectedTables?: SelectedTable[]
		/** Tables that are already added and should show as disabled */
		disabledTables?: SelectedTable[]
	}
	let {
		dbType,
		dbSchema,
		dbTableOpsFactory,
		dbSchemaOps,
		getColDefs,
		dbSupportsSchemas,
		refresh,
		initialSchemaKey,
		initialTableKey,
		selectedSchemaKey = $bindable(undefined),
		selectedTableKey = $bindable(undefined),
		dbSelector,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = []
	}: Props = $props()

	// Helper to check if a table is selected in multi-select mode
	function isTableSelected(schema: string, table: string): boolean {
		return selectedTables.some((t) => t.schema === schema && t.table === table)
	}

	// Helper to check if a table is disabled (already added)
	function isTableDisabled(schema: string, table: string): boolean {
		return disabledTables.some((t) => t.schema === schema && t.table === table)
	}

	// Toggle table selection in multi-select mode
	function toggleTableSelection(schema: string, table: string) {
		if (isTableDisabled(schema, table)) return

		const idx = selectedTables.findIndex((t) => t.schema === schema && t.table === table)
		if (idx >= 0) {
			selectedTables = selectedTables.filter((_, i) => i !== idx)
		} else {
			selectedTables = [...selectedTables, { schema, table }]
		}
	}

	// Get tables for a schema (filtered by search)
	function getTablesForSchema(schema: string): string[] {
		const tables = Object.keys(dbSchema.schema[schema] ?? {})
		if (search) {
			return tables.filter((t) => t.toLowerCase().includes(search.toLowerCase())).sort()
		}
		return tables.sort()
	}

	// Check if all selectable tables in a schema are selected
	function isSchemaFullySelected(schema: string): boolean {
		const tables = getTablesForSchema(schema)
		if (tables.length === 0) return false
		const selectableTables = tables.filter((t) => !isTableDisabled(schema, t))
		if (selectableTables.length === 0) return true // All disabled means "fully selected"
		return selectableTables.every((t) => isTableSelected(schema, t))
	}

	// Check if some (but not all) tables in a schema are selected
	function isSchemaPartiallySelected(schema: string): boolean {
		const tables = getTablesForSchema(schema)
		const selectableTables = tables.filter((t) => !isTableDisabled(schema, t))
		const selectedCount = selectableTables.filter((t) => isTableSelected(schema, t)).length
		return selectedCount > 0 && selectedCount < selectableTables.length
	}

	// Toggle all tables in a schema
	function toggleSchemaSelection(schema: string) {
		const tables = getTablesForSchema(schema)
		const selectableTables = tables.filter((t) => !isTableDisabled(schema, t))

		if (isSchemaFullySelected(schema)) {
			// Deselect all selectable tables in this schema
			selectedTables = selectedTables.filter((t) => t.schema !== schema)
		} else {
			// Select all selectable tables in this schema
			const newSelections = selectableTables
				.filter((t) => !isTableSelected(schema, t))
				.map((t) => ({ schema, table: t }))
			selectedTables = [...selectedTables, ...newSelections]
		}
	}

	let schemaKeys = $derived(Object.keys(dbSchema.schema ?? {}))
	let search = $state('')
	let selected: {
		schemaKey?: undefined | string
		tableKey?: undefined | string
	} = $state({})

	$effect(() => {
		if (!selected.schemaKey && schemaKeys.length) {
			let schemaKey =
				initialSchemaKey ??
				('public' in dbSchema.schema ? 'public' : 'dbo' in dbSchema.schema ? 'dbo' : schemaKeys[0])
			let tableKey =
				initialTableKey && dbSchema.schema?.[schemaKey]?.[initialTableKey]
					? initialTableKey
					: undefined
			selected = { schemaKey, tableKey }
		}
	})

	// Sync selected state with bindable props
	$effect(() => {
		if (selected.schemaKey) {
			selectedSchemaKey = selected.schemaKey
		}
		if (selected.tableKey) {
			selectedTableKey = selected.tableKey
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

	let dbTableEditorState:
		| { open: boolean; alterTableKey?: undefined }
		| { open: true; alterTableKey: string } = $state({
		open: false
	})
	let dbTableEditorAlterTableData = resource(
		() => dbTableEditorState.alterTableKey,
		async (tableKey) => {
			if (!tableKey) return
			return await dbSchemaOps.onFetchTableEditorDefinition({
				table: tableKey,
				schema: selected.schemaKey
			})
		}
	)

	let newSchemaDialogOpen = $state(false)
	let newSchemaName = $state('')

	// Check if the sanitized schema name already exists
	const sanitizedNewSchemaName = $derived(
		newSchemaName
			.trim()
			.toLowerCase()
			.replace(/[^a-zA-Z0-9_]/g, '')
	)
	const schemaAlreadyExists = $derived(
		sanitizedNewSchemaName !== '' && schemaKeys.includes(sanitizedNewSchemaName)
	)
</script>

<Splitpanes>
	<Pane size={24} class="relative flex flex-col">
		<div class="mx-3 mt-3 flex flex-col gap-2">
			{#if dbSelector}
				{@render dbSelector()}
			{/if}
			{#if dbSupportsSchemas && !multiSelectMode}
				<Select
					bind:value={selected.schemaKey}
					items={safeSelectItems(schemaKeys)}
					transformInputSelectedText={(s) => `Schema: ${s}`}
					RightIcon={ChevronDownIcon}
					placeholder="Search or create schema..."
					showPlaceholderOnOpen
					onCreateItem={(schema) => {
						schema = schema
							.trim()
							.toLowerCase()
							.replace(/[^a-zA-Z0-9_]/g, '')
						askingForConfirmation = {
							confirmationText: `Create ${schema}`,
							type: 'reload',
							title: `This will run 'CREATE SCHEMA ${schema}' on your database. Are you sure ?`,
							open: true,
							onConfirm: async () => {
								askingForConfirmation && (askingForConfirmation.loading = true)
								try {
									await dbSchemaOps.onCreateSchema({ schema })
									refresh?.()
									selected.schemaKey = schema
								} finally {
									askingForConfirmation = undefined
								}
							}
						}
					}}
				/>
			{/if}
			<ClearableInput bind:value={search} placeholder="Search table..." />
		</div>
		<div class="overflow-x-clip overflow-y-auto relative mt-3 border-y flex-1">
			{#if multiSelectMode}
				<!-- Multi-select mode: show all schemas with their tables -->
				{#if dbSupportsSchemas}
					<!-- New schema button -->
					<button
						class="w-full text-sm font-medium flex gap-2 items-center h-9 cursor-pointer pl-3 pr-1 hover:bg-gray-500/10 border-b border-surface-secondary text-tertiary"
						onclick={() => (newSchemaDialogOpen = true)}
					>
						<Plus class="shrink-0" size={14} />
						<span class="text-xs">New schema</span>
					</button>
				{/if}
				{#each schemaKeys as schemaKey}
					{@const schemaTables = getTablesForSchema(schemaKey)}
					{@const isFullySelected = isSchemaFullySelected(schemaKey)}
					{@const isPartiallySelected = isSchemaPartiallySelected(schemaKey)}
					{@const hasNoTables = schemaTables.length === 0}
					<!-- Schema header with checkbox (or just label if empty) -->
					<div
						class="group w-full text-sm font-medium flex gap-2 items-center h-9 cursor-pointer pl-3 pr-1 hover:bg-gray-500/10 border-b border-surface-secondary"
						role="button"
						tabindex="0"
						onclick={() => {
							if (!hasNoTables) {
								toggleSchemaSelection(schemaKey)
							}
						}}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								if (!hasNoTables) {
									toggleSchemaSelection(schemaKey)
								}
							}
						}}
					>
						{#if hasNoTables}
							<!-- Empty schema: no checkbox, just indent space -->
							<span class="shrink-0 w-4"></span>
						{:else}
							<span class="shrink-0">
								<input
									type="checkbox"
									checked={isFullySelected}
									indeterminate={isPartiallySelected}
									class="w-4 h-4 cursor-pointer"
									onclick={(e) => e.stopPropagation()}
									onchange={() => toggleSchemaSelection(schemaKey)}
								/>
							</span>
						{/if}
						<span class="truncate text-ellipsis grow text-left text-tertiary text-xs"
							>{schemaKey}</span
						>
						<span class="text-2xs text-tertiary mr-2 group-hover:hidden">
							{schemaTables.length}
						</span>
						<!-- Delete schema button (on hover) -->
						<button
							class="hidden group-hover:flex p-1 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition-colors mr-1"
							title="Delete schema"
							onclick={(e) => {
								e.stopPropagation()
								askingForConfirmation = {
									title: `Are you sure you want to delete schema "${schemaKey}"? This will drop all tables in this schema. This action is irreversible.`,
									confirmationText: 'Drop schema',
									open: true,
									onConfirm: async () => {
										askingForConfirmation && (askingForConfirmation.loading = true)
										try {
											await dbSchemaOps.onDeleteSchema({ schema: schemaKey })
											refresh?.()
											sendUserToast(`Schema '${schemaKey}' deleted successfully`)
										} catch (e) {
											let msg: string | undefined = (e as Error).message
											if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
											sendUserToast(msg ?? 'Action failed!', true)
										}
										askingForConfirmation = undefined
									}
								}
							}}
						>
							<Trash2Icon size={12} class="text-red-500" />
						</button>
					</div>
					<!-- Tables under this schema -->
					{#each schemaTables as tableKey}
						{@const isDisabled = isTableDisabled(schemaKey, tableKey)}
						{@const isChecked = isTableSelected(schemaKey, tableKey) || isDisabled}
						{@const isCurrentPreview =
							selected.schemaKey === schemaKey && selected.tableKey === tableKey}
						<div
							class={'group w-full text-sm font-normal flex gap-2 items-center h-8 cursor-pointer pl-7 pr-1 ' +
								(isCurrentPreview ? 'bg-gray-500/25' : 'hover:bg-gray-500/10') +
								(isDisabled ? ' opacity-50' : '')}
							role="button"
							tabindex="0"
							onclick={() => {
								selected.schemaKey = schemaKey
								selected.tableKey = tableKey
								toggleTableSelection(schemaKey, tableKey)
							}}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									selected.schemaKey = schemaKey
									selected.tableKey = tableKey
									toggleTableSelection(schemaKey, tableKey)
								}
							}}
						>
							<span class="shrink-0">
								<input
									type="checkbox"
									checked={isChecked}
									disabled={isDisabled}
									class="w-4 h-4 cursor-pointer"
									onclick={(e) => e.stopPropagation()}
									onchange={() => toggleTableSelection(schemaKey, tableKey)}
								/>
							</span>
							<Table2 class="text-primary shrink-0" size={14} />
							<p class="truncate text-ellipsis grow text-left text-emphasis text-xs">{tableKey}</p>
							<!-- Delete table button (on hover) -->
							<button
								class="hidden group-hover:flex p-1 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition-colors mr-1"
								title="Delete table"
								onclick={(e) => {
									e.stopPropagation()
									askingForConfirmation = {
										title: `Are you sure you want to delete table "${tableKey}"? This action is irreversible.`,
										confirmationText: 'Drop table',
										open: true,
										onConfirm: async () => {
											askingForConfirmation && (askingForConfirmation.loading = true)
											try {
												await dbSchemaOps.onDelete({ tableKey, schema: schemaKey })
												refresh?.()
												sendUserToast(`Table '${tableKey}' deleted successfully`)
											} catch (e) {
												let msg: string | undefined = (e as Error).message
												if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
												sendUserToast(msg ?? 'Action failed!', true)
											}
											askingForConfirmation = undefined
										}
									}
								}}
							>
								<Trash2Icon size={12} class="text-red-500" />
							</button>
						</div>
					{/each}
					<!-- New table button for this schema -->
					<button
						class="w-full text-sm font-normal flex gap-2 items-center h-8 cursor-pointer pl-7 pr-1 hover:bg-gray-500/10 text-tertiary"
						onclick={() => {
							selected.schemaKey = schemaKey
							dbTableEditorState = { open: true }
						}}
					>
						<Plus class="shrink-0" size={14} />
						<span class="text-xs">New table</span>
					</button>
				{/each}
			{:else}
				<!-- Normal mode: show tables for selected schema -->
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
									displayName: 'Delete table',
									icon: Trash2Icon,
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
								},
								// Only support "Alter table" for PostgreSQL for now
								...(dbType == 'postgresql'
									? [
											{
												displayName: 'Alter table',
												icon: EditIcon,
												action: () => {
													dbTableEditorState = {
														open: true,
														alterTableKey: tableKey
													}
												}
											}
										]
									: [])
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
			{/if}
		</div>
		{#if !multiSelectMode}
			<Button
				on:click={() => (dbTableEditorState = { open: true })}
				wrapperClasses="mx-2 my-2 text-sm"
				startIcon={{ icon: Plus }}
				variant={tableKeys.length === 0 ? 'accent' : 'default'}
			>
				New table
			</Button>
		{/if}
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

<Drawer
	size="600px"
	open={dbTableEditorState.open}
	on:close={() => (dbTableEditorState = { open: false })}
>
	<DrawerContent
		on:close={() => (dbTableEditorState = { open: false })}
		title={dbTableEditorState.alterTableKey
			? `Alter ${dbTableEditorState.alterTableKey}`
			: 'Create a new table'}
	>
		{#key dbTableEditorState.alterTableKey}
			{#if !dbTableEditorState.alterTableKey || dbTableEditorAlterTableData.current}
				<DbTableEditor
					confirmBtnText={dbTableEditorState.alterTableKey
						? `Alter ${dbTableEditorState.alterTableKey}`
						: 'Create table'}
					{dbSchema}
					currentSchema={selected.schemaKey}
					initialValues={dbTableEditorAlterTableData.current}
					onConfirm={async (values) => {
						if (dbTableEditorState.alterTableKey && dbTableEditorAlterTableData.current) {
							let diff = diffCreateTableValues(dbTableEditorAlterTableData.current, values)
							await dbSchemaOps.onAlter({ schema: selected.schemaKey, values: diff })
						} else {
							await dbSchemaOps.onCreate({ values, schema: selected.schemaKey })
						}
						refresh?.()
						sendUserToast(
							dbTableEditorState.alterTableKey
								? dbTableEditorState.alterTableKey + ' updated!'
								: values.name + ' created!'
						)
						dbTableEditorState = { open: false }
					}}
					{dbType}
					previewSql={(values) => {
						if (dbTableEditorState.alterTableKey && dbTableEditorAlterTableData.current) {
							let diff = diffCreateTableValues(dbTableEditorAlterTableData.current, values)
							return dbSchemaOps.previewAlterSql({ values: diff, schema: selected.schemaKey })
						} else {
							return dbSchemaOps.previewCreateSql({ values, schema: selected.schemaKey })
						}
					}}
				/>
			{:else if dbTableEditorAlterTableData.loading}
				<Loader2 class="animate-spin" size={32} />
			{:else}
				<p class="text-sm text-tertiary">Failed to load table definition.</p>
				<p>{dbTableEditorAlterTableData.error}</p>
			{/if}
		{/key}
	</DrawerContent>
</Drawer>

<Drawer
	size="400px"
	open={newSchemaDialogOpen}
	on:close={() => {
		newSchemaDialogOpen = false
		newSchemaName = ''
	}}
>
	<DrawerContent
		on:close={() => {
			newSchemaDialogOpen = false
			newSchemaName = ''
		}}
		title="Create a new schema"
	>
		<div class="flex flex-col gap-4">
			<div>
				<label for="schema-name" class="block text-sm font-medium text-primary mb-1"
					>Schema name</label
				>
				<ClearableInput
					bind:value={newSchemaName}
					placeholder="Enter schema name..."
					autofocus
					on:keydown={(e) => {
						if (e.key === 'Enter' && sanitizedNewSchemaName && !schemaAlreadyExists) {
							askingForConfirmation = {
								confirmationText: `Create ${sanitizedNewSchemaName}`,
								type: 'reload',
								title: `This will run 'CREATE SCHEMA ${sanitizedNewSchemaName}' on your database. Are you sure?`,
								open: true,
								onConfirm: async () => {
									askingForConfirmation && (askingForConfirmation.loading = true)
									try {
										await dbSchemaOps.onCreateSchema({ schema: sanitizedNewSchemaName })
										refresh?.()
										selected.schemaKey = sanitizedNewSchemaName
										newSchemaDialogOpen = false
										newSchemaName = ''
									} finally {
										askingForConfirmation = undefined
									}
								}
							}
						}
					}}
				/>
				{#if schemaAlreadyExists}
					<p class="text-xs text-red-500 mt-1">
						Schema "{sanitizedNewSchemaName}" already exists
					</p>
				{:else}
					<p class="text-xs text-tertiary mt-1">
						Only letters, numbers, and underscores are allowed.
					</p>
				{/if}
			</div>
		</div>
		{#snippet actions()}
			<Button
				color="blue"
				disabled={!sanitizedNewSchemaName || schemaAlreadyExists}
				on:click={() => {
					askingForConfirmation = {
						confirmationText: `Create ${sanitizedNewSchemaName}`,
						type: 'reload',
						title: `This will run 'CREATE SCHEMA ${sanitizedNewSchemaName}' on your database. Are you sure?`,
						open: true,
						onConfirm: async () => {
							askingForConfirmation && (askingForConfirmation.loading = true)
							try {
								await dbSchemaOps.onCreateSchema({ schema: sanitizedNewSchemaName })
								refresh?.()
								selected.schemaKey = sanitizedNewSchemaName
								newSchemaDialogOpen = false
								newSchemaName = ''
							} finally {
								askingForConfirmation = undefined
							}
						}
					}
				}}
			>
				Create schema
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
