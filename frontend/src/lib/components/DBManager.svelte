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
	import { renderDbEqualityFilter, type ColumnDef } from './apps/components/display/dbtable/utils'
	import DBTable, { type DbForeignKeyTarget, type DbRowFilter } from './DBTable.svelte'
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
	import ResizeTransitionWrapper from './common/ResizeTransitionWrapper.svelte'
	import PgAclEditor from './datatableAcl/PgAclEditor.svelte'
	import { favoriteManager } from './sidebar/FavoriteMenu.svelte'
	import DatatableRoleBadge from './DatatableRoleBadge.svelte'
	import type { AclTarget, Asset, DataTableTables } from '$lib/gen'
	import { ADMIN_DATATABLE_ROLE, type DatatableRowAction } from './dbTypes'
	import TextInput from './text_input/TextInput.svelte'
	import Checkbox from './common/checkbox/Checkbox.svelte'

	/** Represents a selected table with its schema */
	export interface SelectedTable {
		/** Absent when the tree has no data table level (a plain database). */
		datatable?: string
		schema: string
		table: string
	}

	/** An action asked for on one data table's row, waiting for the manager to be
	 * connected to that data table. `datatable` is what it was asked for: a switch
	 * that lands anywhere else — because the target failed to load, or because the
	 * user picked another one meanwhile — must not run it there. */
	export type RowAction =
		| { kind: 'create-table'; schema: string }
		| { kind: 'create-schema' }
		| { kind: 'alter-table'; schema: string; table: string }
		| { kind: 'delete-table'; schema: string; table: string }
		| { kind: 'drop-schema'; schema: string }
		| { kind: 'rename-schema'; schema: string }

	export type PendingRowAction = RowAction & { datatable: string }

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
		/** Every data table with its schemas and tables. Present only when the manager
		 * is on a data table — that is what puts a data-table level at the top of the
		 * tree; otherwise the tree starts at schemas. */
		datatableTree?: DataTableTables[]
		datatableTreeLoading?: boolean
		onSelectDatatable?: (datatable: string) => void
		/** Role the manager is connected as; undefined means the data table's default. */
		currentRole?: string
		/** Switch a data table's role from its row badge. */
		onSelectRole?: (datatable: string, role: string) => void
		pendingAction?: PendingRowAction | undefined
		/** Row-menu actions on a data table, run against that row's data table. */
		onDatatableAction?: (datatable: string, action: DatatableRowAction) => void
		/** Workspace the access drawer's own calls run against. */
		workspace?: string
		/** Whether the caller administers data tables here, which is who may edit their roles. */
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
		currentRole,
		onSelectRole,
		pendingAction = $bindable(),
		onDatatableAction,
		workspace,
		canManageDatatable = false,
		multiSelectMode = false,
		selectedTables = $bindable([]),
		disabledTables = [],
		features,
		asset,
		onImport
	}: Props = $props()

	// The engines whose SQL has `ALTER SCHEMA .. RENAME TO`: BigQuery datasets and DuckDB schemas
	// cannot be renamed.
	const SCHEMA_RENAME_DB_TYPES: DbType[] = ['postgresql', 'snowflake']

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

	/** Whether the role this connection uses may create a table in that schema,
	 * or a schema at all. A plain database reports nothing, and there the buttons
	 * stay: hiding them on no information would be worse than a refusal. */
	function canCreateTableIn(datatable: string | undefined, schemaKey: string): boolean {
		const entry = datatableTree?.find((d) => d.datatable_name === datatable)
		if (!entry?.creatable_schemas) return true
		// A schema created since the snapshot is not in it yet; the role just made
		// it, so it can write to it.
		if (!(schemaKey in entry.schemas)) return true
		return entry.creatable_schemas.includes(schemaKey)
	}

	/** Roles and grants only exist on the instance database. Whether the caller may change them
	 * is the access editor's to say: it opens read-only for everyone else. */
	function isInstanceDatatable(datatable: string | undefined): boolean {
		return !!datatableTree?.find((d) => d.datatable_name === datatable)?.instance
	}

	function canCreateSchemaIn(datatable: string | undefined): boolean {
		const entry = datatableTree?.find((d) => d.datatable_name === datatable)
		return entry === undefined || !!entry.can_create_schema
	}

	/** The role a data table row is reached through, and what it can be switched
	 * to. `none` is a data table under roles that grants this caller none of them.
	 * Absent where naming the role says nothing: a data table not under roles, or
	 * one whose single usable role is already `admin`. */
	function roleOf(
		datatable: string
	): { kind: 'role'; role: string; roles: string[] } | { kind: 'none' } | undefined {
		const entry = datatableTree?.find((d) => d.datatable_name === datatable)
		if (!entry?.permissioned) return undefined
		const roles = entry.usable_roles
		if (roles.length === 0) return { kind: 'none' }
		const role = (datatable === currentDatatable ? currentRole : undefined) ?? entry.default_role
		if (roles.length === 1 && roles[0] === ADMIN_DATATABLE_ROLE && role === ADMIN_DATATABLE_ROLE) {
			return undefined
		}
		return { kind: 'role', role, roles }
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
						// A schema that matches keeps all of its tables — the search named
						// the schema, so what is in it is the answer. Copy before sorting:
						// `tables` belongs to the tree snapshot, which is reactive state.
						tables: (matchesSearch(schemaKey) ? [...tables] : tables.filter(matchesSearch)).sort()
					}))
					.filter(
						(sc) => search.trim() === '' || matchesSearch(sc.schemaKey) || sc.tables.length > 0
					)
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
	// What the permissions drawer is open on, if anything: a schema, or a table.
	let aclDrawer = $state<{ datatable: string | undefined; target: AclTarget } | undefined>(
		undefined
	)

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

	const rowChevronClass = (open: boolean) =>
		'shrink-0 text-secondary transition-transform ' + (open ? '' : '-rotate-90')

	/** A favourite says something about the table, so it stays visible; an empty
	 * star is just an affordance and waits for the pointer. */
	const rowStarClass = (path: string) =>
		'-ml-1 w-1 flex shrink-0 transition-opacity ' +
		(favoriteManager.isStarred(path, 'asset') ? '' : 'opacity-0 group-hover:opacity-100')

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
		rowFilter = undefined
		selected = { schemaKey, tableKey }
	}

	/** Run a row action on the data table it belongs to, switching to it first
	 * when it is not the one the manager is connected to. */
	function onDatatable(dt: string | undefined, action: RowAction): boolean {
		if (dt === undefined || dt === currentDatatable) return true
		if ('schema' in action) selectedSchemaKey = action.schema
		if ('table' in action) selectedTableKey = action.table
		pendingAction = { ...action, datatable: dt }
		onSelectDatatable?.(dt)
		return false
	}

	function startCreateTable(dt: string | undefined, schema: string) {
		if (!onDatatable(dt, { kind: 'create-table', schema })) return
		selected = { schemaKey: schema, tableKey: undefined }
		dbTableEditorState = { open: true }
	}

	function startCreateSchema(dt: string | undefined) {
		if (!onDatatable(dt, { kind: 'create-schema' })) return
		schemaDialog = { mode: 'create' }
	}

	function startAlterTable(dt: string | undefined, schema: string, table: string) {
		if (!onDatatable(dt, { kind: 'alter-table', schema, table })) return
		selected = { schemaKey: schema, tableKey: table }
		dbTableEditorState = { open: true, alterTableKey: table }
	}

	function startDeleteTable(dt: string | undefined, schema: string, table: string) {
		if (!onDatatable(dt, { kind: 'delete-table', schema, table })) return
		askingForConfirmation = {
			title: `Are you sure you want to delete ${table} ? This action is irreversible`,
			confirmationText: 'Delete permanently',
			open: true,
			id: 'db-manager-delete-table-confirmation-modal',
			onConfirm: async () => {
				askingForConfirmation && (askingForConfirmation.loading = true)
				try {
					await dbSchemaOps.onDelete({ tableKey: table, schema })
					refresh?.()
					sendUserToast(`Table '${table}' deleted successfully`)
				} catch (e) {
					let msg: string | undefined = (e as any).body ?? (e as Error).message
					if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
					sendUserToast(msg ?? 'Action failed!', true)
				}
				askingForConfirmation = undefined
			}
		}
	}

	function startRenameSchema(dt: string | undefined, schema: string) {
		if (!onDatatable(dt, { kind: 'rename-schema', schema })) return
		schemaDialog = { mode: 'rename', schema }
		newSchemaName = schema
	}

	function startDropSchema(dt: string | undefined, schema: string) {
		if (!onDatatable(dt, { kind: 'drop-schema', schema })) return
		askingForConfirmation = {
			title: `Are you sure you want to drop ${schema} ? Everything in it goes with it, and this action is irreversible`,
			confirmationText: 'Drop permanently',
			open: true,
			id: 'db-manager-drop-schema-confirmation-modal',
			onConfirm: async () => {
				askingForConfirmation && (askingForConfirmation.loading = true)
				try {
					await dbSchemaOps.onDeleteSchema({ schema })
					refresh?.()
					sendUserToast(`Schema '${schema}' dropped successfully`)
				} catch (e) {
					let msg: string | undefined = (e as any).body ?? (e as Error).message
					if (typeof msg !== 'string') msg = e ? JSON.stringify(e) : undefined
					sendUserToast(msg ?? 'Action failed!', true)
				}
				askingForConfirmation = undefined
			}
		}
	}

	// Finishes an action requested before the switch, now that this component is
	// mounted against the data table it targeted.
	$effect(() => {
		const req = pendingAction
		if (!req) return
		// Landed somewhere else: the target may have failed to load, or the user
		// may have moved on. Either way this action was asked for on another
		// database, and dropping a schema is not a thing to do by approximation.
		// `undefined` is somewhere else too — a plain postgres resource or a
		// DuckLake — so the two must be equal, not merely not-known-to-differ.
		if (req.datatable !== currentDatatable) {
			pendingAction = undefined
			return
		}
		// Creating a schema is the one action a data table with none can still
		// take, so it must not wait on a schema being there.
		if (req.kind === 'create-schema') {
			pendingAction = undefined
			schemaDialog = { mode: 'create' }
			return
		}
		if (!schemaKeys.length) return
		pendingAction = undefined
		if (!schemaKeys.includes(req.schema)) return
		if (req.kind === 'create-table') startCreateTable(undefined, req.schema)
		else if (req.kind === 'drop-schema') startDropSchema(undefined, req.schema)
		else if (req.kind === 'rename-schema') startRenameSchema(undefined, req.schema)
		else if (req.kind === 'alter-table') startAlterTable(undefined, req.schema, req.table)
		else startDeleteTable(undefined, req.schema, req.table)
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

	// Set by "Go to row" on a foreign-keyed cell; pinned to the table it was
	// created for so a schema change can't carry it onto an unrelated table.
	let rowFilter: (DbRowFilter & { tableKey: string }) | undefined = $state()
	let activeRowFilter = $derived(rowFilter?.tableKey === tableKey ? rowFilter : undefined)
	let whereClause = $derived(
		activeRowFilter
			? renderDbEqualityFilter(activeRowFilter.column, activeRowFilter.value, dbType)
			: undefined
	)

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
		selectTable(currentDatatable, schemaKey, table)
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

	// Naming a schema: a new one, or a new name for one that exists.
	let schemaDialog = $state<{ mode: 'create' } | { mode: 'rename'; schema: string } | undefined>(
		undefined
	)
	let newSchemaName = $state('')

	/** What the drawer is about, for its title. */
	function aclLabel(target: AclTarget | undefined): string {
		if (!target) return ''
		switch (target.kind) {
			case 'table':
				return `${target.schema}.${target.table}`
			case 'schema':
				return target.schema
			default:
				return 'database'
		}
	}

	function closeSchemaDialog() {
		schemaDialog = undefined
		newSchemaName = ''
	}

	// Check if the sanitized schema name already exists
	const sanitizedNewSchemaName = $derived.by(() => {
		let s = newSchemaName.trim().replace(/[^a-zA-Z0-9_]/g, '')
		if (dbType === 'snowflake') s = s.toUpperCase()
		return s
	})
	const schemaAlreadyExists = $derived(
		sanitizedNewSchemaName !== '' &&
			schemaKeys
				.filter((s) => !(schemaDialog?.mode === 'rename' && s === schemaDialog.schema))
				.map((s) => s.toLowerCase())
				.includes(sanitizedNewSchemaName.toLowerCase())
	)

	/** The statement the dialog is about to run, which is also what it asks about. */
	const schemaStatement = $derived(
		schemaDialog?.mode === 'rename'
			? `ALTER SCHEMA ${schemaDialog.schema} RENAME TO ${sanitizedNewSchemaName}`
			: `CREATE SCHEMA ${sanitizedNewSchemaName}`
	)

	const canSubmitSchemaName = $derived(
		!!sanitizedNewSchemaName &&
			!schemaAlreadyExists &&
			(schemaDialog?.mode !== 'rename' || sanitizedNewSchemaName !== schemaDialog.schema)
	)

	function submitSchemaName() {
		const dialog = schemaDialog
		if (!dialog || !canSubmitSchemaName) return
		const name = sanitizedNewSchemaName
		askingForConfirmation = {
			confirmationText: dialog.mode === 'rename' ? `Rename to ${name}` : `Create ${name}`,
			type: 'reload',
			title: `This will run '${schemaStatement}' on your database. Are you sure?`,
			open: true,
			id: 'db-schema-name-confirmation-modal',
			onConfirm: async () => {
				askingForConfirmation && (askingForConfirmation.loading = true)
				try {
					if (dialog.mode === 'rename') {
						await dbSchemaOps.onRenameSchema({ schema: dialog.schema, newSchema: name })
					} else {
						await dbSchemaOps.onCreateSchema({ schema: name })
					}
					refresh?.()
					selected.schemaKey = name
					closeSchemaDialog()
				} finally {
					askingForConfirmation = undefined
				}
			}
		}
	}

	let _dbTable: DBTable | undefined = $state()
	export const dbTable = () => _dbTable
</script>

<Splitpanes>
	<Pane size={28} class="relative flex flex-col">
		<div class="mx-3 mt-3 flex flex-col gap-2">
			<TextInput bind:value={search} inputProps={{ placeholder: 'Search table or schema...' }} />
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
					{@const hasMenu = !multiSelectMode && onDatatableAction !== undefined}
					{@const roleInfo = roleOf(root.datatable)}
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
						<ChevronDownIcon class={rowChevronClass(dtOpen)} size={14} />
						<DatabaseIcon class="shrink-0" size={14} />
						<span class="truncate text-ellipsis text-left text-xs">{root.datatable}</span>
						{#if root.datatable === currentDatatable}
							<!-- Several data tables are listed, but only one is the one being
							     queried; the tree would otherwise not say which. -->
							<span
								class="shrink-0 w-1.5 h-1.5 rounded-full bg-green-400"
								title="The data table this manager is connected to"
							></span>
						{/if}
						{#if roleInfo?.kind === 'role'}
							{@const dt = root.datatable}
							<DatatableRoleBadge
								role={roleInfo.role}
								roles={roleInfo.roles}
								onSelect={(role) => onSelectRole?.(dt, role)}
							/>
						{:else if roleInfo?.kind === 'none'}
							<span
								class="shrink-0 text-2xs text-tertiary"
								title="This data table is under roles, and none of them is granted to you. Ask an admin of the workspace that governs it."
							>
								No usable role
							</span>
						{/if}
						<div class="grow"></div>
						<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-2">
							{#if hasMenu}
								{@const dt = root.datatable}
								<DropdownV2
									enableFlyTransition
									items={() => [
										{
											displayName: 'Migrations',
											icon: HistoryIcon,
											action: () => onDatatableAction?.(dt, 'migrations')
										},
										...(canManageDatatable && isInstanceDatatable(dt)
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
									btnId={'db-manager-datatable-actions-' + onlyAlphaNumAndUnderscore(dt)}
								/>
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
								<ChevronDownIcon class={rowChevronClass(schemaOpen)} size={14} />
								<FolderIcon class="shrink-0" size={14} />
								<span class="truncate text-ellipsis grow text-left text-xs">{sc.schemaKey}</span>
								<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-2">
									{#if !multiSelectMode}
										<DropdownV2
											enableFlyTransition
											items={() => [
												...(isInstanceDatatable(root.datatable)
													? [
															{
																displayName: 'Access',
																icon: KeyRoundIcon,
																action: () =>
																	(aclDrawer = {
																		datatable: root.datatable,
																		target: { kind: 'schema', schema: sc.schemaKey }
																	})
															}
														]
													: []),
												...(SCHEMA_RENAME_DB_TYPES.includes(dbType)
													? [
															{
																displayName: 'Rename schema',
																icon: EditIcon,
																action: () => startRenameSchema(root.datatable, sc.schemaKey)
															}
														]
													: []),
												{
													displayName: 'Drop schema',
													icon: Trash2Icon,
													type: 'delete',
													action: () => startDropSchema(root.datatable, sc.schemaKey)
												}
											]}
											btnId={'db-manager-schema-actions-' + onlyAlphaNumAndUnderscore(sc.schemaKey)}
										/>
									{/if}
								</div>
							</button>
						{/if}
						<!-- Opening a schema slides its tables in rather than snapping them. -->
						<ResizeTransitionWrapper vertical innerClass="w-full">
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
									{@const hasMenu = !multiSelectMode}
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
										<span class="shrink-0 w-3.5"></span>
										<Table2 class="shrink-0" size={14} />
										<p class="db-manager-table-key truncate text-ellipsis text-left text-xs">
											{tableKey}
										</p>
										{#if asset}
											{@const starPath = tableAssetPath(root.datatable, sc.schemaKey, tableKey)}
											<span class={rowStarClass(starPath)}>
												<Star size={14} kind="asset" path={starPath} />
											</span>
										{/if}
										<div class="grow"></div>
										<div class="relative shrink-0 w-6 h-8 flex items-center justify-end mr-2">
											{#if hasMenu}
												<DropdownV2
													enableFlyTransition
													items={() => [
														...(isInstanceDatatable(root.datatable)
															? [
																	{
																		displayName: 'Access',
																		icon: KeyRoundIcon,
																		action: () =>
																			(aclDrawer = {
																				datatable: root.datatable,
																				target: {
																					kind: 'table',
																					schema: sc.schemaKey,
																					table: tableKey
																				}
																			})
																	}
																]
															: []),
														{
															displayName: 'Delete table',
															icon: Trash2Icon,
															action: () => startDeleteTable(root.datatable, sc.schemaKey, tableKey)
														},
														{
															displayName: 'Alter table',
															icon: EditIcon,
															action: () => startAlterTable(root.datatable, sc.schemaKey, tableKey)
														}
													]}
													btnId={'db-manager-table-actions-' + onlyAlphaNumAndUnderscore(tableKey)}
												/>
											{/if}
										</div>
									</button>
								{/each}
								{#if canCreateTableIn(root.datatable, sc.schemaKey)}
									<button
										class={'w-full text-xs font-normal flex gap-2 items-center h-8 cursor-pointer pr-1 hover:bg-gray-500/10 text-secondary ' +
											tableIndent}
										onclick={() => startCreateTable(root.datatable, sc.schemaKey)}
									>
										<Plus class="shrink-0" size={14} />
										<span class="text-xs">New table</span>
									</button>
								{/if}
							{/if}
						</ResizeTransitionWrapper>
					{/each}
					{#if dbSupportsSchemas && search.trim() === '' && canCreateSchemaIn(root.datatable)}
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
	<Drawer open={!!aclDrawer} size="900px" on:close={() => (aclDrawer = undefined)}>
		<DrawerContent
			title="Access — {aclLabel(aclDrawer?.target)}"
			on:close={() => (aclDrawer = undefined)}
			tooltip="Who owns this, and what each role may do with it."
		>
			{#if aclDrawer && workspace}
				{@const dt = aclDrawer.datatable ?? currentDatatable}
				{#if dt}
					{#key `${dt}~${JSON.stringify(aclDrawer.target)}`}
						<PgAclEditor {workspace} datatable={dt} target={aclDrawer.target} />
					{/key}
				{/if}
			{/if}
		</DrawerContent>
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

<Drawer size="400px" open={!!schemaDialog} on:close={closeSchemaDialog}>
	<DrawerContent
		on:close={closeSchemaDialog}
		title={schemaDialog?.mode === 'rename'
			? `Rename ${schemaDialog.schema}`
			: 'Create a new schema'}
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
						if (e.key === 'Enter') submitSchemaName()
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
			<Button variant="accent" disabled={!canSubmitSchemaName} on:click={submitSchemaName}>
				{schemaDialog?.mode === 'rename' ? 'Rename schema' : 'Create schema'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
