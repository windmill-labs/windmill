<script lang="ts">
	import { superadmin, userStore, type DBSchema } from '$lib/stores'
	import {
		ChevronDownIcon,
		EditIcon,
		Loader2,
		Plus,
		Network,
		Table2,
		Trash2Icon,
		UploadIcon
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { ClearableInput, Drawer, DrawerContent } from './common'
	import { sendUserToast } from '$lib/toast'
	import { renderDbEqualityFilter, type ColumnDef } from './apps/components/display/dbtable/utils'
	import DBTable, { type DbForeignKeyTarget, type DbRowFilter } from './DBTable.svelte'
	import type { IDbSchemaOps, IDbTableOps } from './dbOps'
	import DropdownV2 from './DropdownV2.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import Button from './common/button/Button.svelte'
	import DbTableEditor from './DBTableEditor.svelte'
	import type { DbType } from './dbTypes'
	import Portal from './Portal.svelte'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { untrack, type Snippet } from 'svelte'
	import {
		dbSupportsTransactionalDdl,
		diffTableEditorValues
	} from './apps/components/display/dbtable/queries/alterTable'
	import { resource } from 'runed'
	import { capitalize, onlyAlphaNumAndUnderscore, pluralize } from '$lib/utils'
	import type { DbFeatures } from './apps/components/display/dbtable/dbFeatures'
	import Star from './Star.svelte'
	import type { Asset } from '$lib/gen'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import DbSchemaDiagram from './dbdiagram/DbSchemaDiagram.svelte'
	import type { DbRelation } from './dbRelations'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	/** Represents a selected table with its schema */
	export interface SelectedTable {
		schema: string
		table: string
	}

	/** The right pane's content: the rows of one table, or the schema diagram. */
	export type DbManagerViewMode = 'data' | 'diagram'

	type Props = {
		dbType: DbType
		dbSchema: DBSchema
		dbSupportsSchemas: boolean
		databaseIsEmpty?: boolean
		colDefs: Record<string, ColumnDef[]> | undefined
		dbTableOpsFactory: (params: {
			colDefs: ColumnDef[]
			tableKey: string
			/** Raw SQL predicate AND-ed into the reads (already escaped). */
			whereClause?: string
		}) => IDbTableOps
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
		features?: DbFeatures
		asset?: Asset
		onImport?: (mode: 'schema_and_data' | 'schema_only') => void
	}
	let {
		dbType,
		dbSchema,
		dbTableOpsFactory,
		dbSchemaOps,
		dbSupportsSchemas,
		databaseIsEmpty,
		colDefs,
		refresh,
		initialSchemaKey,
		initialTableKey,
		selectedSchemaKey = $bindable(undefined),
		selectedTableKey = $bindable(undefined),
		dbSelector,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = [],
		features,
		asset,
		onImport
	}: Props = $props()

	let viewMode = $state<DbManagerViewMode>('data')

	// PostgreSQL is the only database whose foreign keys can be read for the whole
	// database in one query; the others would need one job per table. A caller
	// already using the sidebar checkboxes to collect tables keeps them.
	let supportsDiagram = $derived(dbType === 'postgresql' && !multiSelectMode)

	// The tables drawn on the diagram. Kept apart from `selectedTables` so that
	// checking a table to see it in the diagram can never add it to whatever the
	// caller's multi-select is collecting.
	let diagramTables = $state<SelectedTable[]>([])

	let showsCheckboxes = $derived(multiSelectMode || viewMode === 'diagram')
	let checkedTables = $derived(viewMode === 'diagram' ? diagramTables : selectedTables)

	function setCheckedTables(tables: SelectedTable[]) {
		if (viewMode === 'diagram') diagramTables = tables
		else selectedTables = tables
	}

	// Helper to check if a table is selected in multi-select mode
	function isTableSelected(schema: string, table: string): boolean {
		return checkedTables.some((t) => t.schema === schema && t.table === table)
	}

	// Helper to check if a table is disabled (already added). Only the caller's
	// selection has tables that are already spoken for.
	function isTableDisabled(schema: string, table: string): boolean {
		return (
			viewMode !== 'diagram' && disabledTables.some((t) => t.schema === schema && t.table === table)
		)
	}

	// Toggle table selection in multi-select mode
	function toggleTableSelection(schema: string, table: string) {
		if (isTableDisabled(schema, table)) return

		const idx = checkedTables.findIndex((t) => t.schema === schema && t.table === table)
		if (idx >= 0) {
			setCheckedTables(checkedTables.filter((_, i) => i !== idx))
		} else {
			setCheckedTables([...checkedTables, { schema, table }])
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
			setCheckedTables(checkedTables.filter((t) => t.schema !== schema))
		} else {
			// Select all selectable tables in this schema
			const newSelections = selectableTables
				.filter((t) => !isTableSelected(schema, t))
				.map((t) => ({ schema, table: t }))
			setCheckedTables([...checkedTables, ...newSelections])
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
				('public' in dbSchema.schema
					? 'public'
					: 'dbo' in dbSchema.schema
						? 'dbo'
						: 'main' in dbSchema.schema
							? 'main'
							: schemaKeys[0])
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

	// Set by "Go to row" on a foreign-keyed cell; pinned to the table it was
	// created for so a schema change can't carry it onto an unrelated table.
	let rowFilter: (DbRowFilter & { tableKey: string }) | undefined = $state()
	let activeRowFilter = $derived(rowFilter?.tableKey === tableKey ? rowFilter : undefined)
	let whereClause = $derived(
		activeRowFilter
			? renderDbEqualityFilter(activeRowFilter.column, activeRowFilter.value, dbType)
			: undefined
	)

	function selectTable(schemaKey: string | undefined, table: string) {
		rowFilter = undefined
		selected = { schemaKey, tableKey: table }
	}

	/** Where a foreign key's `schema.table` target lives in the sidebar, or
	 * undefined when it cannot be opened from here. */
	function resolveForeignKeyTarget(
		targetTable: string
	): { schemaKey: string; table: string } | undefined {
		const parts = targetTable.split('.')
		const table = parts[parts.length - 1]
		const qualifier = parts.length > 1 ? parts.slice(0, -1).join('.') : undefined
		// Without schema support the sidebar browses the connection's default
		// schema only, and unqualified reads would hit a same-named local table.
		if (!dbSupportsSchemas && qualifier && qualifier !== selected.schemaKey) return undefined
		const schemaKey = dbSupportsSchemas && qualifier ? qualifier : selected.schemaKey
		if (!schemaKey || !(table in (dbSchema.schema[schemaKey] ?? {}))) return undefined
		return { schemaKey, table }
	}

	function goToRow(target: DbForeignKeyTarget) {
		const resolved = resolveForeignKeyTarget(target.table)
		if (!resolved) {
			sendUserToast(`Table ${target.table} cannot be opened from this schema`, true)
			return
		}
		if (renderDbEqualityFilter(target.column, target.value, dbType) === undefined) {
			sendUserToast('This value cannot be used as a filter', true)
			return
		}
		const { schemaKey, table } = resolved
		selectTable(schemaKey, table)
		rowFilter = {
			tableKey: dbSupportsSchemas ? `${schemaKey}.${table}` : table,
			column: target.column,
			value: target.value
		}
	}

	// The result carries the table it was fetched for: `resource` keeps the
	// previous value while refetching, and a stale list would decorate the new
	// table's same-named columns as foreign keys.
	let foreignKeys = resource(
		[() => selected.tableKey, () => selected.schemaKey, () => colDefs],
		async ([table, schema], _prev, { signal }) => {
			if (!table) return undefined
			const forTableKey = dbSupportsSchemas && schema ? `${schema}.${table}` : table
			const fks =
				features?.foreignKeys === false
					? []
					: await dbSchemaOps.onFetchForeignKeys({ table, schema })
			// A newer selection started meanwhile: an AbortError keeps this result
			// out of `current`, where it would shadow the newer table's keys.
			if (signal.aborted) throw new DOMException('Superseded', 'AbortError')
			return { tableKey: forTableKey, foreignKeys: fks }
		}
	)
	// Only keys whose target the sidebar can open get the "Go to row" affordance.
	let currentForeignKeys = $derived.by(() => {
		const fetched = foreignKeys.current
		if (!fetched || fetched.tableKey !== tableKey) return undefined
		return fetched.foreignKeys.filter(
			(fk) => fk.targetTable && resolveForeignKeyTarget(fk.targetTable) !== undefined
		)
	})

	let askingForConfirmation:
		| (ConfirmationModal['$$prop_def'] & { onConfirm: () => void })
		| undefined = $state()

	let dbTableEditorState:
		| { open: boolean; alterTableKey?: undefined }
		| { open: true; alterTableKey: string } = $state({
		open: false
	})
	let dbTableEditorAlterTableData = resource(
		[() => dbTableEditorState.alterTableKey, () => colDefs],
		async ([table]) => {
			if (!table) return
			let tableKey2 =
				dbSupportsSchemas && selected.schemaKey ? `${selected.schemaKey}.${table}` : table
			if (!colDefs?.[tableKey2]) return
			return await dbSchemaOps.onFetchTableEditorDefinition({
				table: table,
				schema: selected.schemaKey,
				colDefs: colDefs[tableKey2]
			})
		}
	)

	let newSchemaDialogOpen = $state(false)
	let newSchemaName = $state('')

	// Check if the sanitized schema name already exists
	const sanitizedNewSchemaName = $derived.by(() => {
		let s = newSchemaName.trim().replace(/[^a-zA-Z0-9_]/g, '')
		if (dbType === 'snowflake') s = s.toUpperCase()
		return s
	})
	const schemaAlreadyExists = $derived(
		sanitizedNewSchemaName !== '' &&
			schemaKeys.map((s) => s.toLowerCase()).includes(sanitizedNewSchemaName.toLowerCase())
	)

	// Fetched once for the whole database rather than per table: the diagram needs
	// every relation at once, and the per-table query would be one job each.
	let relationsError = $state<string | undefined>(undefined)
	let relations = resource(
		[() => viewMode, () => colDefs],
		async ([mode]): Promise<DbRelation[]> => {
			// Keeps what was fetched when leaving the diagram, so coming back to it
			// doesn't queue the query again.
			if (mode !== 'diagram') return relations.current ?? []
			relationsError = undefined
			try {
				return await dbSchemaOps.onFetchAllForeignKeys()
			} catch (e) {
				relationsError = (e as any)?.body ?? (e as Error)?.message ?? String(e)
				return []
			}
		}
	)

	// Opening the diagram on an empty canvas would make it look broken, so the
	// current schema is drawn to start with — unless it is big enough that drawing
	// all of it is a choice the user should make.
	const DIAGRAM_AUTOSELECT_LIMIT = 40
	$effect(() => {
		if (viewMode !== 'diagram' || diagramTables.length) return
		const schemaKey = untrack(() => selected.schemaKey)
		if (!schemaKey) return
		const tables = Object.keys(dbSchema.schema[schemaKey] ?? {})
		if (tables.length > DIAGRAM_AUTOSELECT_LIMIT) return
		diagramTables = tables.map((table) => ({ schema: schemaKey, table }))
	})

	let _dbTable: DBTable | undefined = $state()
	export const dbTable = () => _dbTable
</script>

<Splitpanes>
	<Pane size={24} class="relative flex flex-col">
		<div class="mx-3 mt-3 flex flex-col gap-2">
			{#if dbSelector}
				{@render dbSelector()}
			{/if}
			{#if supportsDiagram}
				<ToggleButtonGroup
					bind:selected={viewMode}
					noWFull
					onSelected={(v) => logFeatureUsage('db_manager', 'view_mode', { key: v })}
				>
					{#snippet children({ item })}
						<ToggleButton value="data" label="Data" icon={Table2} {item} />
						<ToggleButton value="diagram" label="Diagram" icon={Network} {item} />
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			{#if dbSupportsSchemas && !showsCheckboxes}
				<Select
					bind:value={selected.schemaKey}
					items={safeSelectItems(schemaKeys)}
					id="db-schema-select"
					transformInputSelectedText={(s) => `Schema: ${s}`}
					RightIcon={ChevronDownIcon}
					placeholder="Search or create schema..."
					showPlaceholderOnOpen
					onCreateItem={(schema) => {
						schema = schema.trim().replace(/[^a-zA-Z0-9_]/g, '')
						if (dbType === 'snowflake') schema = schema.toUpperCase()
						askingForConfirmation = {
							confirmationText: `Create ${schema}`,
							type: 'reload',
							title: `This will run 'CREATE SCHEMA ${schema}' on your database. Are you sure ?`,
							open: true,
							id: 'db-create-schema-confirmation-modal',
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
			{#if showsCheckboxes}
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
											let msg: string | undefined = (e as any).body ?? (e as Error).message
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
								selectTable(schemaKey, tableKey)
								toggleTableSelection(schemaKey, tableKey)
							}}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') {
									selectTable(schemaKey, tableKey)
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
												let msg: string | undefined = (e as any).body ?? (e as Error).message
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
					<!-- PLACEHOLDER -->
					<button
						class={'w-full text-sm font-normal flex gap-2 items-center h-10 cursor-pointer pl-3 pr-1 ' +
							(selected.tableKey === tableKey ? 'bg-surface-secondary' : 'hover:bg-surface-hover')}
						onclick={() => selectTable(selected.schemaKey, tableKey)}
					>
						{#if asset}
							<Star
								kind="asset"
								path={`${asset.kind}://${asset.path == 'main' ? '' : asset.path}/${selected.schemaKey}.${tableKey}`}
							/>
						{:else}
							<Table2 class="text-primary shrink-0" size={14} />
						{/if}

						<p
							class="db-manager-table-key truncate text-ellipsis grow text-left text-emphasis text-xs"
						>
							{tableKey}
						</p>
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
											id: 'db-manager-delete-table-confirmation-modal',
											onConfirm: async () => {
												askingForConfirmation && (askingForConfirmation.loading = true)
												try {
													await dbSchemaOps.onDelete({ tableKey, schema: selected.schemaKey })
													refresh?.()
													sendUserToast(`Table '${tableKey}' deleted successfully`)
												} catch (e) {
													let msg: string | undefined = (e as any).body ?? (e as Error).message
													if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
													sendUserToast(msg ?? 'Action failed!', true)
												}
												askingForConfirmation = undefined
											}
										})
								},
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
							]}
							class="w-fit"
							btnId={'db-manager-table-actions-' + onlyAlphaNumAndUnderscore(tableKey)}
						/>
					</button>
				{/each}
			{/if}
		</div>
		{#if !showsCheckboxes}
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
	<Pane class={viewMode === 'diagram' ? '' : 'p-3 pt-1'}>
		{#if viewMode === 'diagram'}
			<DbSchemaDiagram
				{dbSchema}
				{colDefs}
				selectedTables={diagramTables}
				relations={relations.current ?? []}
				loading={relations.loading}
				error={relationsError}
				onOpenTable={({ schema, table }) => {
					viewMode = 'data'
					selectTable(schema, table)
				}}
			/>
		{:else if tableKey && colDefs?.[tableKey]?.length}
			{@const dbTableOps = dbTableOpsFactory({ colDefs: colDefs[tableKey], tableKey, whereClause })}
			<DBTable
				{dbTableOps}
				foreignKeys={currentForeignKeys}
				onGoToRow={goToRow}
				rowFilter={activeRowFilter}
				onClearRowFilter={() => (rowFilter = undefined)}
				bind:this={_dbTable}
			/>
		{:else if databaseIsEmpty}
			<div class="h-full w-full center-center flex-col gap-4">
				<span class="text-hint">Database is empty</span>
				{#if onImport}
					<div class="flex gap-4">
						<button
							onclick={() => onImport('schema_only')}
							class="hover:opacity-70 transition-opacity rounded-md border aspect-square w-52 gap-4 p-4 center-center flex-col"
						>
							<UploadIcon size={64} class="text-secondary" />
							<span class="text-center font-normal text-sm text-secondary">
								Import schema from database
							</span>
						</button>
						{#if !!$userStore?.is_admin || !!$superadmin}
							<button
								onclick={() => onImport('schema_and_data')}
								class="hover:opacity-70 transition-opacity rounded-md border aspect-square w-52 gap-4 p-4 center-center flex-col"
							>
								<UploadIcon size={64} class="text-secondary" />
								<span class="text-center font-normal text-sm text-secondary">
									Import schema and data from database
								</span>
							</button>
						{/if}
					</div>
				{/if}
			</div>
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
		id="db-table-editor-drawer"
		on:close={() => (dbTableEditorState = { open: false })}
		title={dbTableEditorState.alterTableKey
			? `Alter ${dbTableEditorState.alterTableKey}`
			: 'Create a new table'}
	>
		{#key dbTableEditorState.alterTableKey}
			{#if !dbTableEditorState.alterTableKey || dbTableEditorAlterTableData.current}
				<DbTableEditor
					{features}
					{dbSchema}
					currentSchema={selected.schemaKey}
					initialValues={dbTableEditorAlterTableData.current}
					onConfirm={async ({ values }) => {
						if (dbTableEditorState.alterTableKey && dbTableEditorAlterTableData.current) {
							let diff = diffTableEditorValues(dbTableEditorAlterTableData.current, values)
							// Reverse diff (new → old) so the migration's down undoes the alter.
							let reverse = diffTableEditorValues(values, dbTableEditorAlterTableData.current)
							await dbSchemaOps.onAlter({ schema: selected.schemaKey, values: diff, reverse })
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
					computePreview={async ({ values }) => {
						if (dbTableEditorState.alterTableKey && dbTableEditorAlterTableData.current) {
							let diff = diffTableEditorValues(dbTableEditorAlterTableData.current, values)
							let sql = await dbSchemaOps.previewAlterSql({
								values: diff,
								schema: selected.schemaKey
							})
							let alert = !dbSupportsTransactionalDdl(dbType)
								? {
										title: capitalize(dbType) + ' does not support transactional DDL',
										body: 'Any of these statements failing may leave your database in an intermediate state.'
									}
								: undefined
							return { sql, ...(alert ? { alert } : {}) }
						} else {
							let sql = await dbSchemaOps.previewCreateSql({ values, schema: selected.schemaKey })
							return { sql }
						}
					}}
					computeBtnProps={({ values }) => {
						if (dbTableEditorState.alterTableKey && dbTableEditorAlterTableData.current) {
							let diff = diffTableEditorValues(dbTableEditorAlterTableData.current, values)
							if (!diff.operations.length) {
								return { text: 'No changes detected', disabled: true }
							}
							return {
								text: `Alter table (${pluralize(diff.operations.length, 'change')} detected)`
							}
						} else {
							return { text: 'Create table' }
						}
					}}
				/>
			{:else if dbTableEditorAlterTableData.loading || !colDefs}
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
								id: 'db-create-schema-confirmation-modal',
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
						id: 'db-create-schema-confirmation-modal',
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
