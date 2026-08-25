<script lang="ts">
	import { superadmin, userStore, type DBSchema } from '$lib/stores'
	import {
		ChevronDownIcon,
		EditIcon,
		Loader2,
		Plus,
		Table2,
		Database as DatabaseIcon,
		Folder as FolderIcon,
		History as HistoryIcon,
		KeyRound as KeyRoundIcon,
		Download as DownloadIcon,
		Trash2Icon,
		UploadIcon
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
	import {
		dbSupportsTransactionalDdl,
		diffTableEditorValues
	} from './apps/components/display/dbtable/queries/alterTable'
	import { resource } from 'runed'
	import type { Snippet } from 'svelte'
	import { capitalize, onlyAlphaNumAndUnderscore, pluralize } from '$lib/utils'
	import type { DbFeatures } from './apps/components/display/dbtable/dbFeatures'
	import Star from './Star.svelte'
	import type { Asset, DataTableTables } from '$lib/gen'
	import type { DatatableRowAction } from './dbTypes'
	import TextInput from './text_input/TextInput.svelte'

	/** Represents a selected table with its schema */
	export interface SelectedTable {
		schema: string
		table: string
	}

	type Props = {
		dbType: DbType
		dbSchema: DBSchema
		dbSupportsSchemas: boolean
		databaseIsEmpty?: boolean
		colDefs: Record<string, ColumnDef[]> | undefined
		dbTableOpsFactory: (params: { colDefs: ColumnDef[]; tableKey: string }) => IDbTableOps
		dbSchemaOps: IDbSchemaOps
		refresh?: () => void
		initialSchemaKey?: string
		initialTableKey?: string
		selectedSchemaKey?: string | undefined
		selectedTableKey?: string | undefined
		/** Every data table with its schemas and tables. Present only when the manager
		 * is on a data table — that is what puts a data-table level at the top of the
		 * tree; otherwise the tree starts at schemas. */
		/** Multi-select pickers still choose their data table with a Select above the
		 * list; the tree below is the navigator for the manager's normal mode. */
		dbSelector?: Snippet<[]>
		datatableTree?: DataTableTables[]
		datatableTreeLoading?: boolean
		onSelectDatatable?: (datatable: string) => void
		/** Row-menu actions on a data table, run against that row's data table. */
		onDatatableAction?: (datatable: string, action: DatatableRowAction) => void
		canManageDatatable?: boolean
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
		datatableTree,
		datatableTreeLoading,
		onSelectDatatable,
		onDatatableAction,
		canManageDatatable = false,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = [],
		features,
		asset,
		onImport
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

	// --- Left-pane tree ---------------------------------------------------------
	// Levels: data table -> schema -> table. The top two collapse away on their own
	// terms: no `datatableTree` means this is not a data table, and a database
	// without schemas has nothing to put between a data table and its tables.
	const currentDatatable = $derived(asset?.kind === 'datatable' ? asset.path : undefined)

	/** Tables per schema for a data table, as `schema -> table[]`. */
	function schemasOf(datatable: string | undefined): Record<string, string[]> {
		// The open data table reads from `dbSchema`, which is refetched after a DDL;
		// the tree snapshot is not, so using it here would hide a table until the
		// next full reload.
		if (datatable === undefined || datatable === currentDatatable) {
			return Object.fromEntries(
				Object.entries(dbSchema.schema ?? {}).map(([sk, tables]) => [sk, Object.keys(tables ?? {})])
			)
		}
		return datatableTree?.find((d) => d.datatable_name === datatable)?.schemas ?? {}
	}

	function errorOf(datatable: string): string | undefined {
		return datatableTree?.find((d) => d.datatable_name === datatable)?.error
	}

	const matchesSearch = (t: string) => t.toLowerCase().includes(search.trim().toLowerCase())

	/** The tree as rendered: only nodes with a matching descendant survive a search. */
	let treeRoots = $derived.by(() => {
		const datatables = datatableTree
			? datatableTree.map((d) => d.datatable_name)
			: [undefined as string | undefined]
		return datatables
			.map((dt) => {
				const schemas = Object.entries(schemasOf(dt))
					.map(([schemaKey, tables]) => ({
						schemaKey,
						tables: tables.filter(matchesSearch).sort()
					}))
					.filter((sc) => search.trim() === '' || sc.tables.length > 0)
				schemas.sort((a, b) => a.schemaKey.localeCompare(b.schemaKey))
				return { datatable: dt, schemas, error: dt ? errorOf(dt) : undefined }
			})
			.filter(
				// A search narrows the tree to what matched; a data table with no match
				// left in it would otherwise sit there as an empty row.
				(root) => search.trim() === '' || root.schemas.length > 0
			)
	})

	// Explicit open/closed choices, over a default rule. Storing only the
	// overrides is what lets the current data table and selected schema — which
	// default to open — actually be folded; a plain "expanded" set could never
	// close them, since the default would keep winning.
	// Schema-level permissions are not built yet; the drawer is the shell the row
	// menu already opens onto.
	let schemaPermissionsOpen = $state(false)

	let expandOverrides = $state<Map<string, boolean>>(new Map())
	const nodeKey = (dt: string | undefined, schemaKey?: string) =>
		`${dt ?? ''}${schemaKey === undefined ? '' : `/${schemaKey}`}`

	function defaultExpanded(dt: string | undefined, schemaKey?: string): boolean {
		if (dt !== undefined && dt !== currentDatatable) return false
		return schemaKey === undefined || schemaKey === selected.schemaKey
	}

	function isExpanded(dt: string | undefined, schemaKey?: string): boolean {
		// A search narrows the tree to what matched, so everything left is shown.
		if (search.trim() !== '') return true
		return expandOverrides.get(nodeKey(dt, schemaKey)) ?? defaultExpanded(dt, schemaKey)
	}

	function toggle(dt: string | undefined, schemaKey?: string) {
		const key = nodeKey(dt, schemaKey)
		const open = expandOverrides.get(key) ?? defaultExpanded(dt, schemaKey)
		const next = new Map(expandOverrides)
		next.set(key, !open)
		expandOverrides = next
	}

	// Row actions stay out of the way until you are on the row — or it is the one
	// you are looking at, where the menu is part of the current context.
	const rowActionsClass = (current: boolean) =>
		'absolute right-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100 ' +
		(current ? 'opacity-100' : 'opacity-0')

	// The chevron is the resting state of that slot: it gives way to the menu
	// rather than sitting beside it.
	const rowChevronClass = (current: boolean, open: boolean) =>
		'absolute right-0 pointer-events-none text-secondary transition-all group-hover:opacity-0 ' +
		(current ? 'opacity-0 ' : 'opacity-100 ') +
		(open ? '' : '-rotate-90')

	/** Every row in the tree reads the same way: what you are looking at now is
	 * emphasized, everything else recedes. */
	const rowText = (current: boolean) =>
		current ? 'text-primary font-semibold' : 'text-secondary font-normal'

	/** Reveal a node, dropping a stale "closed" that would hide a new selection. */
	function reveal(dt: string | undefined, schemaKey?: string) {
		const next = new Map(expandOverrides)
		next.delete(nodeKey(dt))
		next.delete(nodeKey(dt, schemaKey))
		expandOverrides = next
	}

	function selectTable(dt: string | undefined, schemaKey: string, tableKey: string) {
		if (dt !== undefined && dt !== currentDatatable) {
			// Switching data table re-mounts this component against the new one, so
			// the target has to travel through the bound keys the parent keeps —
			// local state here is about to be thrown away.
			selectedSchemaKey = schemaKey
			selectedTableKey = tableKey
			onSelectDatatable?.(dt)
			return
		}
		reveal(dt, schemaKey)
		selected = { schemaKey, tableKey }
	}

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

	let _dbTable: DBTable | undefined = $state()
	export const dbTable = () => _dbTable
</script>

<Splitpanes>
	<Pane size={24} class="relative flex flex-col">
		<div class="mx-3 mt-3 flex flex-col gap-2">
			{#if multiSelectMode && dbSelector}
				{@render dbSelector()}
			{/if}
			<TextInput bind:value={search} inputProps={{ placeholder: 'Search table...' }} />
		</div>
		<div class="overflow-x-clip overflow-y-auto relative mt-1.5 flex-1">
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
				<!-- Normal mode: data table -> schema -> table, each level dropping out
				     when it has nothing to say (no data table / no schemas). -->
				{#if datatableTreeLoading && (datatableTree?.length ?? 0) === 0}
					<div class="flex items-center gap-2 text-tertiary p-3">
						<Loader2 class="animate-spin" size={14} />
						<span class="text-xs">Loading...</span>
					</div>
				{/if}
				{#each treeRoots as root (root.datatable ?? '')}
					{@const dtOpen = isExpanded(root.datatable)}
					{#if root.datatable !== undefined}
						<button
							class={'group w-full text-sm flex gap-2 items-center h-8 cursor-pointer pl-2 pr-1 hover:bg-gray-500/10 ' +
								rowText(root.datatable === currentDatatable)}
							onclick={() => toggle(root.datatable)}
						>
							<DatabaseIcon class="shrink-0" size={14} />
							<span class="truncate text-ellipsis grow text-left text-xs">{root.datatable}</span>
							<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-1">
								{#if onDatatableAction}
									{@const dt = root.datatable}
									<DropdownV2
										items={() => [
											{
												displayName: 'Migrations',
												icon: HistoryIcon,
												action: () => onDatatableAction?.(dt, 'migrations')
											},
											...(canManageDatatable
												? [
														{
															displayName: 'Roles',
															icon: KeyRoundIcon,
															action: () => onDatatableAction?.(dt, 'roles')
														}
													]
												: []),
											{
												displayName: 'Export',
												icon: DownloadIcon,
												action: () => onDatatableAction?.(dt, 'export')
											},
											{
												displayName: 'Import',
												icon: UploadIcon,
												action: () => onDatatableAction?.(dt, 'import')
											}
										]}
										class="-mr-2 {rowActionsClass(root.datatable === currentDatatable)}"
										btnId={'db-manager-datatable-actions-' + onlyAlphaNumAndUnderscore(dt)}
									/>
								{/if}
								<ChevronDownIcon
									class={rowChevronClass(root.datatable === currentDatatable, dtOpen)}
									size={14}
								/>
							</div>
						</button>
					{/if}
					{#if dtOpen}
						{#if root.error}
							<p class="text-xs text-red-400 px-3 py-2">{root.error}</p>
						{/if}
						{#each root.schemas as sc (sc.schemaKey)}
							{@const schemaOpen = isExpanded(root.datatable, sc.schemaKey)}
							{@const indent = root.datatable !== undefined ? 'pl-6' : 'pl-2'}
							{#if dbSupportsSchemas}
								<button
									class={'group w-full text-sm flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 ' +
										indent +
										' ' +
										rowText(
											root.datatable === currentDatatable && sc.schemaKey === selected.schemaKey
										)}
									onclick={() => toggle(root.datatable, sc.schemaKey)}
								>
									<FolderIcon class="shrink-0" size={14} />
									<span class="truncate text-ellipsis grow text-left text-xs">{sc.schemaKey}</span>
									<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-1">
										<DropdownV2
											items={() => [
												{
													displayName: 'Permissions',
													icon: KeyRoundIcon,
													action: () => (schemaPermissionsOpen = true)
												}
											]}
											class="-mr-2 {rowActionsClass(
												root.datatable === currentDatatable && sc.schemaKey === selected.schemaKey
											)}"
											btnId={'db-manager-schema-actions-' + onlyAlphaNumAndUnderscore(sc.schemaKey)}
										/>
										<ChevronDownIcon
											class={rowChevronClass(
												root.datatable === currentDatatable && sc.schemaKey === selected.schemaKey,
												schemaOpen
											)}
											size={14}
										/>
									</div>
								</button>
							{/if}
							{#if schemaOpen || !dbSupportsSchemas}
								{@const tableIndent = dbSupportsSchemas
									? root.datatable !== undefined
										? 'pl-12'
										: 'pl-8'
									: root.datatable !== undefined
										? 'pl-8'
										: 'pl-3'}
								{#each sc.tables as tableKey (tableKey)}
									{@const isSelected =
										root.datatable === currentDatatable &&
										selected.schemaKey === sc.schemaKey &&
										selected.tableKey === tableKey}
									<button
										class={'group w-full text-sm flex gap-2 items-center h-8 cursor-pointer pr-1 ' +
											tableIndent +
											' ' +
											rowText(isSelected) +
											' ' +
											(isSelected ? 'bg-surface-secondary' : 'hover:bg-surface-hover')}
										onclick={() => selectTable(root.datatable, sc.schemaKey, tableKey)}
									>
										{#if asset}
											<Star
												kind="asset"
												path={`${asset.kind}://${asset.path == 'main' ? '' : asset.path}/${sc.schemaKey}.${tableKey}`}
											/>
										{:else}
											<Table2 class="shrink-0" size={14} />
										{/if}
										<p class="db-manager-table-key truncate text-ellipsis grow text-left text-xs">
											{tableKey}
										</p>
										{#if root.datatable === currentDatatable || root.datatable === undefined}
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
																		await dbSchemaOps.onDelete({
																			tableKey,
																			schema: sc.schemaKey
																		})
																		refresh?.()
																		sendUserToast(`Table '${tableKey}' deleted successfully`)
																	} catch (e) {
																		let msg: string | undefined =
																			(e as any).body ?? (e as Error).message
																		if (typeof msg !== 'string')
																			msg = e ? JSON.stringify(e) : undefined
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
															selected = { schemaKey: sc.schemaKey, tableKey }
															dbTableEditorState = { open: true, alterTableKey: tableKey }
														}
													}
												]}
												class={rowActionsClass(isSelected)}
												btnId={'db-manager-table-actions-' + onlyAlphaNumAndUnderscore(tableKey)}
											/>
										{/if}
									</button>
								{/each}
								{#if root.datatable === currentDatatable || root.datatable === undefined}
									<button
										class={'w-full text-sm font-normal flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 text-tertiary ' +
											tableIndent}
										onclick={() => {
											selected = { schemaKey: sc.schemaKey, tableKey: undefined }
											dbTableEditorState = { open: true }
										}}
									>
										<Plus class="shrink-0" size={14} />
										<span class="text-xs">New table</span>
									</button>
								{/if}
							{/if}
						{/each}
						{#if dbSupportsSchemas && (root.datatable === currentDatatable || root.datatable === undefined) && search.trim() === ''}
							<button
								class={'w-full text-sm font-normal flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 text-tertiary ' +
									(root.datatable !== undefined ? 'pl-6' : 'pl-2')}
								onclick={() => (newSchemaDialogOpen = true)}
							>
								<Plus class="shrink-0" size={14} />
								<span class="text-xs">New schema</span>
							</button>
						{/if}
					{/if}
				{/each}
			{/if}
		</div>
	</Pane>
	<Pane class="p-3 pt-1">
		{#if tableKey && colDefs?.[tableKey]?.length}
			{@const dbTableOps = dbTableOpsFactory({ colDefs: colDefs[tableKey], tableKey })}
			<DBTable {dbTableOps} bind:this={_dbTable} />
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
	<Drawer bind:open={schemaPermissionsOpen} size="900px">
		<DrawerContent title="Schema permissions" on:close={() => (schemaPermissionsOpen = false)} />
	</Drawer>

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
