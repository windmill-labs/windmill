<script lang="ts">
	import { enterpriseLicense, superadmin, userStore, workspaceStore } from '$lib/stores'
	import { WorkspaceService, type DataTableTables } from '$lib/gen'
	import { listUsableDatatableRoles } from './datatableUsableRoles'
	import Button from './common/button/Button.svelte'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Select from './select/Select.svelte'
	import {
		ArrowLeft,
		Copy,
		Download,
		Expand,
		Minimize,
		Network,
		RefreshCcw,
		Table2,
		Upload
	} from 'lucide-svelte'
	import DBManagerContent from './DBManagerContent.svelte'
	import type { DbManagerViewMode, PendingRowAction } from './DBManager.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { getDbType } from './dbOps'
	import DataTableMigrationsButton from './workspaceSettings/DataTableMigrationsButton.svelte'
	import DataTablePermissionsButton from './workspaceSettings/DataTablePermissionsButton.svelte'
	import { resource } from 'runed'
	import { tick, untrack } from 'svelte'
	import type { DbManagerUriState } from './dbManagerDrawerModel.svelte'
	import {
		ADMIN_DATATABLE_ROLE,
		datatableNameTakesRole,
		defaultMigrationRole,
		type DatatableRowAction
	} from './dbTypes'
	import ResourcePicker from './ResourcePicker.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { useDbManagerTag } from './dbManagerTag.svelte'
	import DbWorkerTagButton from './DbWorkerTagButton.svelte'

	interface Props {
		uriState: DbManagerUriState
		/** Z-index offset for the drawer, useful when opening from within modals */
		offset?: number
	}

	let { uriState, offset = 0 }: Props = $props()

	let open = $derived(uriState.open)

	// The workspace the drawer's DB operations run against — the acting workspace of
	// the editor that opened it (set via openDrawer), else the nav workspace.
	let ws = $derived(uriState.workspace ?? $workspaceStore)

	// A create started on a data table other than the current one: survives the
	// re-mount the switch causes.
	let pendingAction = $state<PendingRowAction | undefined>(undefined)

	// Read once through primitives: the getters return values of a freshly parsed URL, which
	// changes on every table click, and the listings below must not refetch for that.
	const selectedDatatable = $derived(uriState.selectedDatatable)
	const selectedRole = $derived(uriState.selectedRole)

	// Roles the caller may use, to settle the role before anything connects. Offering only
	// these is a convenience: the server refuses any other.
	const usableRoles = resource(
		() => [ws, selectedDatatable] as const,
		async ([workspace, datatable]) => {
			if (!workspace || !datatable) return undefined
			try {
				return {
					workspace,
					datatable,
					...(await listUsableDatatableRoles(workspace, datatable))
				}
			} catch (e) {
				// Never leave the drawer waiting on this: fall back to the
				// unpermissioned shape so it opens and the server picks the role.
				console.error('Failed to load datatable roles:', e)
				return {
					workspace,
					datatable,
					permissioned: false,
					roles: [],
					default_role: ADMIN_DATATABLE_ROLE
				}
			}
		}
	)

	// A resource keeps its previous value while refetching, and roles are per data table of one
	// workspace: settling from another answer would connect to this data table as a role it may
	// not even have.
	const rolesOfCurrent = $derived(
		usableRoles.current?.datatable === selectedDatatable && usableRoles.current?.workspace === ws
			? usableRoles.current
			: undefined
	)

	// Nothing that connects runs until the role is settled: a first round sent without a role
	// would run — and cache — as whatever the server defaults to.
	const roleSettled = $derived(
		!uriState.isDatatableInput ||
			(rolesOfCurrent !== undefined &&
				(!rolesOfCurrent.permissioned ||
					rolesOfCurrent.roles.length === 0 ||
					selectedRole !== undefined ||
					// Its reference cannot name a role, so it connects as the default one.
					(selectedDatatable !== undefined && !datatableNameTakesRole(selectedDatatable))))
	)

	// Make the role explicit before anything queries the data table, so the URL, the
	// cache and every migration the manager writes name it. A role already in the URL
	// is kept even when it is not usable: the server refuses it, visibly.
	$effect(() => {
		const roles = rolesOfCurrent
		if (
			!roles?.permissioned ||
			selectedRole !== undefined ||
			(selectedDatatable !== undefined && !datatableNameTakesRole(selectedDatatable))
		)
			return
		const effective = roles.roles.includes(roles.default_role) ? roles.default_role : roles.roles[0]
		if (effective) untrack(() => (uriState.selectedRole = effective))
	})

	const contentInput = $derived.by(() => {
		const input = uriState.effectiveInput
		if (input?.type !== 'database' || selectedDatatable === undefined) return input
		const migrationRole = defaultMigrationRole(
			selectedDatatable,
			rolesOfCurrent?.permissioned,
			rolesOfCurrent?.default_role
		)
		return migrationRole === undefined ? input : { ...input, migrationRole }
	})

	// Every data table with its schemas and tables, in one call: this is what the
	// left pane's tree navigates, so it has to cover the data tables the user is
	// not currently on, not just the selected one. The privileges it reports are
	// the connected role's, so the role picked on the open data table is part of
	// what is being asked. Gated on the drawer being open on a data table: this
	// reaches every data table's database in turn, and the component is mounted on
	// every logged-in page.
	let datatablesRun = 0
	const datatables = resource(
		() =>
			[
				open && uriState.isDatatableInput,
				ws,
				selectedDatatable,
				selectedRole,
				roleSettled
			] as const,
		async ([active, workspace, roleFor, role, settled]): Promise<DataTableTables[]> => {
			if (!active || !workspace) return []
			if (!settled) return untrack(() => datatables.current)
			const run = ++datatablesRun
			try {
				const result = await WorkspaceService.listDataTableTables({ workspace, roleFor, role })
				// An answer for a selection that has since changed describes another role.
				return run === datatablesRun ? result : untrack(() => datatables.current)
			} catch (e) {
				console.error('Failed to load datatables:', e)
				return run === datatablesRun ? [] : untrack(() => datatables.current)
			}
		},
		{ initialValue: [] }
	)

	function handleClose() {
		uriState.closeDrawer()
		dbManagerContent?.clearReplResult()
	}

	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	$effect(() => {
		if (!open) {
			expand = false
			uriState.closeDrawer()
			// An action asked for on one data table must not be waiting when the
			// drawer is next opened on another database — or on no data table at
			// all, where nothing would recognise it as foreign.
			pendingAction = undefined
		}
	})

	let dbManagerContent: DBManagerContent | undefined = $state()

	// Per-database worker tag override, remembered across drawer opens.
	const workerTag = useDbManagerTag(
		() => ws,
		() => uriState.effectiveInput
	)

	let hasReplResult = $state(false)

	// Which view the manager shows. Held here, beside the control that switches it
	// and outside the key that remounts the manager, so picking another data table
	// or role stays on the view the user was reading.
	let requestedViewMode = $state<DbManagerViewMode>('data')
	// Only PostgreSQL has a diagram; the manager clamps the mode itself, and this
	// keeps the control off the header for a database that cannot show one.
	let diagramSupported = $derived(
		!!uriState.effectiveInput && getDbType(uriState.effectiveInput) === 'postgresql'
	)

	// Export/Import state
	let exportDrawerOpen = $state(false)
	let exportResult = $state('')
	let importDrawerOpen = $state(false)
	let importLoading = $state(false)
	let importSource = $state<string | undefined>(undefined)
	/** Which database an import writes into; set when driven from a tree row. */
	let importTarget = $state<string | undefined>(undefined)
	let importBehavior = $state<'schema_only' | 'schema_and_data'>('schema_only')

	let isPostgresqlInput = $derived(
		uriState.isDatatableInput ||
			(uriState.input?.type === 'database' && uriState.input.resourceType === 'postgresql')
	)
	let enableImportExport = $derived(isPostgresqlInput)

	function toSourceIdentifier(raw: string): string {
		if (raw.startsWith('datatable://') || raw.startsWith('$res:')) return raw
		return `$res:${raw}`
	}

	function currentSourceIdentifier(): string | undefined {
		const input = uriState.effectiveInput
		if (!input || input.type !== 'database') return undefined
		return toSourceIdentifier(input.resourcePath)
	}

	// The tree's row menus act on the data table of the row that was clicked, which
	// is not necessarily the one currently open — so the target is set first and the
	// headless modals are keyed on it.
	let actionDatatable = $state<string | undefined>(undefined)
	let migrationsModal = $state<DataTableMigrationsButton | undefined>()
	let permissionsDrawer = $state<DataTablePermissionsButton | undefined>()

	async function runDatatableAction(datatable: string, action: DatatableRowAction) {
		actionDatatable = datatable
		// Let the keyed block above mount against the new target before driving it.
		await tick()
		switch (action) {
			case 'migrations':
				migrationsModal?.open()
				break
			case 'roles':
				permissionsDrawer?.open()
				break
			case 'export':
				await handleExportSchema(`datatable://${datatable}`)
				break
			case 'import':
				importTarget = `datatable://${datatable}`
				importDrawerOpen = true
				break
		}
	}

	function refreshManager() {
		dbManagerContent?.refresh()
		dbManagerContent?.dbManager()?.dbTable()?.refresh()
		refreshRoles()
	}

	/** Re-read what the tree and the role picker show: both are answers about the
	 * data table's roles, which the permissions drawer can have just changed. */
	function refreshRoles() {
		datatables.refetch()
		usableRoles.refetch()
	}

	async function handleExportSchema(explicitSource?: string) {
		const source = explicitSource ?? currentSourceIdentifier()
		if (!source || !ws) return
		try {
			exportResult = await WorkspaceService.exportPgSchema({
				workspace: ws,
				requestBody: { source }
			})
			exportDrawerOpen = true
		} catch (e) {
			sendUserToast(`Failed to export schema: ${e}`, true)
		}
	}

	async function handleImportDatabase() {
		if (!importSource || !ws) return
		const target = importTarget ?? currentSourceIdentifier()
		if (!target) return
		importLoading = true
		try {
			await WorkspaceService.importPgDatabase({
				workspace: ws,
				requestBody: {
					source: toSourceIdentifier(importSource),
					target,
					fork_behavior: importBehavior
				}
			})
			sendUserToast('Database import completed successfully')
			importDrawerOpen = false
			importSource = undefined
			dbManagerContent?.refresh()
		} catch (e) {
			sendUserToast(`Failed to import database: ${e}`, true)
		} finally {
			importLoading = false
		}
	}
</script>

<svelte:window bind:innerWidth={windowWidth} />

<Drawer
	bind:open
	size={expand ? `${windowWidth}px` : '1200px'}
	preventEscape
	{offset}
	on:close={handleClose}
>
	<DrawerContent
		title={hasReplResult ? 'Query Result' : 'Database Manager'}
		on:close={() => {
			if (hasReplResult) {
				dbManagerContent?.clearReplResult()
			} else {
				handleClose()
			}
		}}
		CloseIcon={hasReplResult ? ArrowLeft : undefined}
		noPadding
		id="db-manager-drawer"
	>
		{#if contentInput && ws && roleSettled}
			{#key `${selectedDatatable}~${selectedRole ?? ''}`}
				<DBManagerContent
					bind:this={dbManagerContent}
					{requestedViewMode}
					onViewMode={(mode) => (requestedViewMode = mode)}
					input={contentInput}
					workspace={uriState.workspace}
					datatableTree={uriState.isDatatableInput ? datatables.current : undefined}
					datatableTreeLoading={datatables.loading}
					onSelectDatatable={(dt) => (uriState.selectedDatatable = dt)}
					onSelectRole={(dt, role) => {
						// Setting the data table clears the role, so the order matters.
						uriState.selectedDatatable = dt
						uriState.selectedRole = role
					}}
					bind:pendingAction
					canManageDatatable={!!($superadmin || $userStore?.is_admin) &&
						!!$enterpriseLicense &&
						!isCloudHosted()}
					onDatatableAction={runDatatableAction}
					bind:workerTag={() => workerTag.tag, (v) => (workerTag.tag = v)}
					bind:hasReplResult
					bind:selectedSchemaKey={uriState.selectedSchema}
					bind:selectedTableKey={uriState.selectedTable}
					onImport={enableImportExport
						? (mode) => (
								(importTarget = undefined),
								(importDrawerOpen = true),
								(importBehavior = mode)
							)
						: undefined}
				></DBManagerContent>
			{/key}
		{/if}
		{#snippet actions()}
			<!-- A data table exports and imports from its row menu in the tree; a plain
				 database has no tree row to hold them. -->
			{#if enableImportExport && !uriState.isDatatableInput}
				<Button startIcon={{ icon: Download }} onClick={() => handleExportSchema()}>Export</Button>
				<Button
					startIcon={{ icon: Upload }}
					onClick={() => ((importTarget = undefined), (importDrawerOpen = true))}
				>
					Import
				</Button>
			{/if}
			{#if diagramSupported}
				<ToggleButtonGroup
					bind:selected={requestedViewMode}
					noWFull
					onSelected={(v) => logFeatureUsage('db_manager', 'view_mode', { key: v })}
				>
					{#snippet children({ item })}
						<ToggleButton value="data" label="Data" icon={Table2} {item} />
						<ToggleButton value="diagram" label="Diagram" icon={Network} {item} />
					{/snippet}
				</ToggleButtonGroup>
			{/if}
			{#if uriState.effectiveInput && ws}
				<DbWorkerTagButton
					bind:tag={() => workerTag.tag, (v) => (workerTag.tag = v)}
					input={uriState.effectiveInput}
					workspace={ws}
				/>
			{/if}

			<Button
				loading={dbManagerContent?.isLoading() ?? false}
				on:click={refreshManager}
				startIcon={{ icon: RefreshCcw }}
				iconOnly
				title="Refresh"
				size="xs"
				color="light"
			/>

			<Button
				on:click={() => (expand = !expand)}
				startIcon={{ icon: expand ? Minimize : Expand }}
				size="xs"
				color="light"
			/>
		{/snippet}
	</DrawerContent>
</Drawer>

{#if actionDatatable && ws}
	{#key actionDatatable}
		<DataTableMigrationsButton
			bind:this={migrationsModal}
			hideTrigger
			workspace={ws}
			datatable={actionDatatable}
			onSchemaChanged={refreshManager}
		/>
		{#if $enterpriseLicense && !isCloudHosted()}
			<DataTablePermissionsButton
				bind:this={permissionsDrawer}
				hideTrigger
				workspace={ws}
				datatable={actionDatatable}
				onSaved={refreshRoles}
			/>
		{/if}
	{/key}
{/if}

<Drawer bind:open={exportDrawerOpen} size="800px" offset={offset + 1}>
	<DrawerContent title="Export Schemas" on:close={() => (exportDrawerOpen = false)}>
		{#if exportResult}
			<div class="flex flex-col gap-2 h-full relative">
				<pre class="overflow-auto text-xs bg-surface-secondary p-4 rounded flex-1"
					>{exportResult}</pre
				>
				<Button
					size="xs"
					color="light"
					startIcon={{ icon: Copy }}
					wrapperClasses="absolute top-2 right-2"
					btnClasses="bg-surface-tertiary"
					on:click={() => {
						navigator.clipboard.writeText(exportResult)
						sendUserToast('Copied to clipboard')
					}}
				>
					Copy
				</Button>
			</div>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:open={importDrawerOpen} size="600px" offset={offset + 1}>
	<DrawerContent title="Import Database" on:close={() => (importDrawerOpen = false)}>
		<div class="flex flex-col gap-4">
			<Alert type="warning" title="Warning">
				This will import the schemas from the selected source into the current database. Existing
				tables with the same names may be affected.
			</Alert>
			<div class="flex flex-col gap-2">
				<span class="text-sm font-medium">Source database</span>
				<ResourcePicker
					datatableAsPgResource
					bind:value={importSource}
					resourceType="postgresql"
					workspace={ws}
				/>
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-sm font-medium">Import mode</span>
				<Select
					items={[
						{ value: 'schema_only', label: 'Schema only' },
						...(isCloudHosted() || (!$superadmin && !$userStore?.is_admin)
							? []
							: [{ value: 'schema_and_data', label: 'Schema and data' }])
					]}
					bind:value={importBehavior}
				/>
			</div>
			{#if importBehavior === 'schema_and_data'}
				<Alert type="warning" title="Heavy operation">
					Importing schema and data will copy all rows from every table in the source database. This
					may take a long time and use significant storage space depending on the size of the
					source.
				</Alert>
			{/if}
			<Button
				disabled={!importSource}
				loading={importLoading}
				color="red"
				on:click={handleImportDatabase}
			>
				Import {importBehavior === 'schema_and_data' ? 'schemas and data' : 'schemas'} into current database
			</Button>
		</div>
	</DrawerContent>
</Drawer>
