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
	import { capitalize, onlyAlphaNumAndUnderscore, pluralize } from '$lib/utils'
	import type { DbFeatures } from './apps/components/display/dbtable/dbFeatures'
	import Star from './Star.svelte'
	import type { Asset, DataTableTables } from '$lib/gen'
	import type { DatatableRowAction } from './dbTypes'
	import TextInput from './text_input/TextInput.svelte'
	import Checkbox from './common/checkbox/Checkbox.svelte'

	/** Represents a selected table with its schema */
	export interface SelectedTable {
		/** Absent when the tree has no data table level (a plain database). */
		datatable?: string
		schema: string
		table: string
	}

	/** A create started from a row of another data table. Switching data table
	 * re-mounts this component, so the request has to travel through the parent. */
	export type PendingCreate = { kind: 'table'; schema: string } | { kind: 'schema' }

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
		datatableTree?: DataTableTables[]
		datatableTreeLoading?: boolean
		onSelectDatatable?: (datatable: string) => void
		pendingCreate?: PendingCreate | undefined
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
		datatableTree,
		datatableTreeLoading,
		onSelectDatatable,
		pendingCreate = $bindable(undefined),
		onDatatableAction,
		canManageDatatable = false,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = [],
		features,
		asset,
		onImport
	}: Props = $props()

	const sameTable = (a: SelectedTable, b: SelectedTable) =>
		a.datatable === b.datatable && a.schema === b.schema && a.table === b.table

	function isTableSelected(t: SelectedTable): boolean {
		return selectedTables.some((s) => sameTable(s, t))
	}

	/** Already added by the caller: shown ticked and locked. */
	function isTableDisabled(t: SelectedTable): boolean {
		return disabledTables.some((s) => sameTable(s, t))
	}

	function toggleTableSelection(t: SelectedTable) {
		if (isTableDisabled(t)) return
		selectedTables = isTableSelected(t)
			? selectedTables.filter((s) => !sameTable(s, t))
			: [...selectedTables, t]
	}

	/** Every table under a node, as it is currently rendered — so a batch toggle
	 * acts on what the user can see, search filter included. */
	function tablesUnder(datatable: string | undefined, schemaKey?: string): SelectedTable[] {
		return treeRoots
			.filter((r) => r.datatable === datatable)
			.flatMap((r) =>
				r.schemas
					.filter((sc) => schemaKey === undefined || sc.schemaKey === schemaKey)
					.flatMap((sc) => sc.tables.map((table) => ({ datatable, schema: sc.schemaKey, table })))
			)
	}

	/** Tri-state of a node's batch checkbox. Locked tables count as ticked, so a
	 * node whose tables were all already added reads as full rather than empty. */
	function batchState(
		datatable: string | undefined,
		schemaKey?: string
	): { checked: boolean; indeterminate: boolean; disabled: boolean } {
		const tables = tablesUnder(datatable, schemaKey)
		const selectable = tables.filter((t) => !isTableDisabled(t))
		const n = tables.filter((t) => isTableSelected(t) || isTableDisabled(t)).length
		return {
			checked: tables.length > 0 && n === tables.length,
			indeterminate: n > 0 && n < tables.length,
			disabled: selectable.length === 0
		}
	}

	function toggleBatch(datatable: string | undefined, schemaKey?: string) {
		const selectable = tablesUnder(datatable, schemaKey).filter((t) => !isTableDisabled(t))
		if (selectable.every((t) => isTableSelected(t))) {
			selectedTables = selectedTables.filter((s) => !selectable.some((t) => sameTable(s, t)))
		} else {
			const missing = selectable.filter((t) => !isTableSelected(t))
			selectedTables = [...selectedTables, ...missing]
		}
	}

	let schemaKeys = $derived(Object.keys(dbSchema.schema ?? {}))

	// --- Left-pane tree ---------------------------------------------------------
	// Levels: data table -> schema -> table. The top two collapse away on their own
	// terms: no `datatableTree` means this is not a data table, and a database
	// without schemas has nothing to put between a data table and its tables.
	const currentDatatable = $derived(asset?.kind === 'datatable' ? asset.path : undefined)

	/** Favourites are keyed by the table's own asset URI, so a row under another
	 * data table must not borrow the one the manager is currently pointed at. */
	function tableAssetPath(datatable: string | undefined, schemaKey: string, tableKey: string) {
		const kind = datatable !== undefined ? 'datatable' : asset!.kind
		const path = datatable ?? asset!.path
		return `${kind}://${path === 'main' ? '' : path}/${schemaKey}.${tableKey}`
	}

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

	// Row actions stay out of the way until you are on the row.
	const rowActionsClass =
		'absolute right-0 opacity-0 transition-opacity focus-visible:opacity-100 group-hover:opacity-100'

	// The chevron is the resting state of that slot: it gives way to the menu
	// rather than sitting beside it.
	const rowChevronClass = (open: boolean) =>
		'absolute right-0 pointer-events-none text-secondary transition-all opacity-100 ' +
		(multiSelectMode ? '' : 'group-hover:opacity-0 ') +
		(open ? '' : '-rotate-90')

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

	function startCreateTable(dt: string | undefined, schemaKey: string) {
		if (dt !== undefined && dt !== currentDatatable) {
			selectedSchemaKey = schemaKey
			pendingCreate = { kind: 'table', schema: schemaKey }
			onSelectDatatable?.(dt)
			return
		}
		selected = { schemaKey, tableKey: undefined }
		dbTableEditorState = { open: true }
	}

	function startCreateSchema(dt: string | undefined) {
		if (dt !== undefined && dt !== currentDatatable) {
			pendingCreate = { kind: 'schema' }
			onSelectDatatable?.(dt)
			return
		}
		newSchemaDialogOpen = true
	}

	// Finishes a create requested before the switch, now that this component is
	// mounted against the data table it targeted.
	$effect(() => {
		const req = pendingCreate
		if (!req || !schemaKeys.length) return
		pendingCreate = undefined
		if (req.kind === 'schema') {
			newSchemaDialogOpen = true
		} else if (schemaKeys.includes(req.schema)) {
			selected = { schemaKey: req.schema, tableKey: undefined }
			dbTableEditorState = { open: true }
		}
	})

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
			<TextInput bind:value={search} inputProps={{ placeholder: 'Search table...' }} />
		</div>
		<div class="overflow-x-clip overflow-y-auto relative mt-1.5 flex-1">
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
						class="group w-full text-xs font-normal text-primary flex gap-2 items-center h-8 cursor-pointer pl-3 pr-1 hover:bg-gray-500/10"
						onclick={() => toggle(root.datatable)}
					>
						{#if multiSelectMode}
							{@const state = batchState(root.datatable)}
							<Checkbox
								checked={state.checked}
								indeterminate={state.indeterminate}
								disabled={state.disabled}
								onChange={() => toggleBatch(root.datatable)}
								onClick={(e) => e.stopPropagation()}
								class="shrink-0"
							/>
						{/if}
						<DatabaseIcon class="shrink-0" size={14} />
						<span class="truncate text-ellipsis grow text-left text-xs">{root.datatable}</span>
						<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-2">
							<ChevronDownIcon class={rowChevronClass(dtOpen)} size={14} />
							{#if !multiSelectMode}
								{#if onDatatableAction}
									{@const dt = root.datatable}
									<DropdownV2
										enableFlyTransition
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
										class="-mr-2 {rowActionsClass}"
										btnId={'db-manager-datatable-actions-' + onlyAlphaNumAndUnderscore(dt)}
									/>
								{/if}
							{/if}
						</div>
					</button>
				{/if}
				{#if dtOpen}
					{#if root.error}
						<p class="text-xs text-red-400 px-3 py-2">{root.error}</p>
					{/if}
					{#each root.schemas as sc (sc.schemaKey)}
						{@const schemaOpen = isExpanded(root.datatable, sc.schemaKey)}
						{@const indent = root.datatable !== undefined ? 'pl-7' : 'pl-3'}
						{#if dbSupportsSchemas}
							<button
								class={'group w-full text-xs font-normal text-primary flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 ' +
									indent}
								onclick={() => toggle(root.datatable, sc.schemaKey)}
							>
								{#if multiSelectMode}
									{@const state = batchState(root.datatable, sc.schemaKey)}
									<Checkbox
										checked={state.checked}
										indeterminate={state.indeterminate}
										disabled={state.disabled}
										onChange={() => toggleBatch(root.datatable, sc.schemaKey)}
										onClick={(e) => e.stopPropagation()}
										class="shrink-0"
									/>
								{/if}
								<FolderIcon class="shrink-0" size={14} />
								<span class="truncate text-ellipsis grow text-left text-xs">{sc.schemaKey}</span>
								<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-2">
									<ChevronDownIcon class={rowChevronClass(schemaOpen)} size={14} />
									{#if !multiSelectMode}
										<DropdownV2
											enableFlyTransition
											items={() => [
												{
													displayName: 'Permissions',
													icon: KeyRoundIcon,
													action: () => (schemaPermissionsOpen = true)
												}
											]}
											class="-mr-2 {rowActionsClass}"
											btnId={'db-manager-schema-actions-' + onlyAlphaNumAndUnderscore(sc.schemaKey)}
										/>
									{/if}
								</div>
							</button>
						{/if}
						{#if schemaOpen || !dbSupportsSchemas}
							{@const tableIndent = dbSupportsSchemas
								? root.datatable !== undefined
									? 'pl-11'
									: 'pl-7'
								: root.datatable !== undefined
									? 'pl-7'
									: 'pl-3'}
							{#each sc.tables as tableKey (tableKey)}
								{@const entry = {
									datatable: root.datatable,
									schema: sc.schemaKey,
									table: tableKey
								}}
								{@const isSelected =
									root.datatable === currentDatatable &&
									selected.schemaKey === sc.schemaKey &&
									selected.tableKey === tableKey}
								<button
									class={'group w-full text-xs font-normal text-primary flex gap-2 items-center h-8 cursor-pointer pr-1 ' +
										tableIndent +
										' ' +
										(isSelected ? 'bg-surface-secondary' : 'hover:bg-surface-hover')}
									onclick={() => selectTable(root.datatable, sc.schemaKey, tableKey)}
								>
									{#if multiSelectMode}
										<Checkbox
											checked={isTableSelected(entry) || isTableDisabled(entry)}
											disabled={isTableDisabled(entry)}
											title={isTableDisabled(entry) ? 'Already added' : undefined}
											onChange={() => toggleTableSelection(entry)}
											onClick={(e) => e.stopPropagation()}
											class="shrink-0"
										/>
									{/if}
									{#if asset}
										<!-- Star carries its own p-1 for a bigger hit area; pull it back so its
											     glyph lands on the same indent grid as a bare icon. -->
										<span class="-ml-1 flex shrink-0">
											<Star
												kind="asset"
												path={tableAssetPath(root.datatable, sc.schemaKey, tableKey)}
											/>
										</span>
									{:else}
										<Table2 class="shrink-0" size={14} />
									{/if}
									<p class="db-manager-table-key truncate text-ellipsis grow text-left text-xs">
										{tableKey}
									</p>
									{#if !multiSelectMode && (root.datatable === currentDatatable || root.datatable === undefined)}
										<DropdownV2
											enableFlyTransition
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
											class="mr-1 {rowActionsClass}"
											btnId={'db-manager-table-actions-' + onlyAlphaNumAndUnderscore(tableKey)}
										/>
									{/if}
								</button>
							{/each}
							<button
								class={'w-full text-xs font-normal flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 text-secondary ' +
									tableIndent}
								onclick={() => startCreateTable(root.datatable, sc.schemaKey)}
							>
								<Plus class="shrink-0" size={14} />
								<span class="text-xs">New table</span>
							</button>
						{/if}
					{/each}
					{#if dbSupportsSchemas && search.trim() === ''}
						<button
							class={'w-full text-xs font-normal flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 text-secondary ' +
								(root.datatable !== undefined ? 'pl-7' : 'pl-3')}
							onclick={() => startCreateSchema(root.datatable)}
						>
							<Plus class="shrink-0" size={14} />
							<span class="text-xs">New schema</span>
						</button>
					{/if}
				{/if}
			{/each}
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
