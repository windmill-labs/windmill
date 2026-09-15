<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { appDatatableRole, type DataTableRef } from './dataTableRefUtils'
	import { untrack } from 'svelte'
	import { resource } from 'runed'
	import { ArrowLeft, Expand, Minimize, Plus, RefreshCcw } from 'lucide-svelte'
	import DBManagerContent from '../DBManagerContent.svelte'
	import { ADMIN_DATATABLE_ROLE, type DbInput } from '../dbTypes'
	import type { PendingRowAction, SelectedTable } from '../DBManager.svelte'
	import { getRawAppOperatingWorkspace } from './rawAppWorkspace'
	import { useDbManagerTag } from '../dbManagerTag.svelte'
	import DbWorkerTagButton from '../DbWorkerTagButton.svelte'
	import type { DataTableTables } from '$lib/gen'

	const getOpWs = getRawAppOperatingWorkspace()
	let opWs = $derived(getOpWs?.() ?? $workspaceStore)

	interface Props {
		/** `roles` holds, for each added table's data table under roles, the role its tables were
		 * browsed as: the app uses the data table through it from then on. */
		onAdd?: (refs: DataTableRef[], roles: Record<string, string>) => void
		existingRefs?: DataTableRef[]
		/** The role the app uses each data table through, by data table name */
		roles?: Record<string, string>
		/** Z-index offset for the drawer, useful when opening from within modals */
		offset?: number
	}

	let { onAdd, existingRefs = [], roles = undefined, offset = 0 }: Props = $props()

	let open = $state(false)
	let selectedDatatable = $state<string | undefined>(undefined)
	/** Role the manager connects as; undefined means the data table's default. */
	let selectedRole = $state<string | undefined>(undefined)

	// For DB manager
	let dbManagerContent: DBManagerContent | undefined = $state()
	let hasReplResult = $state(false)
	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	// Multi-select mode: selected tables
	let selectedTables = $state<SelectedTable[]>([])
	/** The role each data table's selected tables were browsed as. */
	let browsedRoles = $state<Record<string, string>>({})

	// Survives the re-mount a data table switch causes.
	let pendingAction = $state<PendingRowAction | undefined>(undefined)

	// Selected schema/table from DBManager (for preview)
	let selectedSchemaKey = $state<string | undefined>(undefined)
	let selectedTableKey = $state<string | undefined>(undefined)
	// What the manager opens on, set only when it (re-)mounts: the live selection above changes
	// on every click, and feeding it to the input would reload the whole manager each time.
	let openSchemaKey = $state<string | undefined>(undefined)
	let openTableKey = $state<string | undefined>(undefined)

	// Load available datatables from workspace
	const datatables = resource<string[]>([], async () => {
		if (!opWs) return []
		try {
			return (await WorkspaceService.listDataTables({ workspace: opWs })).map((d) => d.name)
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})

	const usableRoles = resource(
		() => [open, opWs, selectedDatatable] as const,
		async ([isOpen, workspace, datatable]) => {
			if (!isOpen || !workspace || !datatable) return undefined
			try {
				return {
					datatable,
					...(await WorkspaceService.listUsableDatatableRoles({
						workspace,
						datatableName: datatable
					}))
				}
			} catch (e) {
				// Opens anyway: without a role the server connects as the default and says so if
				// that is refused.
				console.error('Failed to load datatable roles:', e)
				return {
					datatable,
					permissioned: false,
					roles: [] as string[],
					default_role: ADMIN_DATATABLE_ROLE
				}
			}
		}
	)

	// A resource keeps its previous value while it refetches, and roles are per data table.
	const rolesOfCurrent = $derived(
		usableRoles.current?.datatable === selectedDatatable ? usableRoles.current : undefined
	)

	// Mounting the manager fires its first queries, so it waits for the role: a round sent
	// without one runs, and caches, as whatever the server defaults to.
	const roleSettled = $derived(
		selectedDatatable === undefined ||
			(rolesOfCurrent !== undefined &&
				(!rolesOfCurrent.permissioned ||
					rolesOfCurrent.roles.length === 0 ||
					selectedRole !== undefined))
	)

	$effect(() => {
		const current = rolesOfCurrent
		if (!current?.permissioned || selectedRole !== undefined) return
		const effective = current.roles.includes(current.default_role)
			? current.default_role
			: current.roles[0]
		if (effective) untrack(() => (selectedRole = effective))
	})

	// Every data table with its schemas and tables: the tree is the picker. The privileges it
	// reports are the connected role's, so the role picked on the open data table is asked too.
	// Waits for the role like the manager does, and drops an answer for a selection that has
	// since changed: it would describe another role.
	let datatableTreeRun = 0
	const datatableTree = resource(
		() => [open, opWs, selectedDatatable, selectedRole, roleSettled] as const,
		async ([isOpen, workspace, roleFor, role, settled]): Promise<DataTableTables[]> => {
			if (!isOpen || !workspace) return []
			if (!settled) return untrack(() => datatableTree.current)
			const run = ++datatableTreeRun
			try {
				const result = await WorkspaceService.listDataTableTables({
					workspace,
					roleFor: role ? roleFor : undefined,
					role
				})
				return run === datatableTreeRun ? result : untrack(() => datatableTree.current)
			} catch (e) {
				console.error('Failed to load datatable tables:', e)
				return run === datatableTreeRun ? [] : untrack(() => datatableTree.current)
			}
		},
		{ initialValue: [] }
	)

	/** Remembers the role the connected data table's selected tables were seen through. */
	function recordBrowsedRole() {
		const dt = selectedDatatable
		if (!dt || !selectedRole || !rolesOfCurrent?.permissioned) return
		if (selectedTables.some((t) => (t.datatable ?? dt) === dt)) {
			browsedRoles = { ...browsedRoles, [dt]: selectedRole }
		}
	}

	function selectDatatable(datatable: string, role?: string) {
		recordBrowsedRole()
		// A row clicked under another data table has just set the selection it should open on.
		openSchemaKey = selectedSchemaKey
		openTableKey = selectedTableKey
		selectedDatatable = datatable
		// A data table the app already uses through a role opens as that role.
		selectedRole = role ?? appDatatableRole(roles, datatable)
	}

	export function openDrawer() {
		selectedSchemaKey = undefined
		selectedTableKey = undefined
		selectDatatable(datatables.current.includes('main') ? 'main' : datatables.current[0])
		selectedTables = []
		browsedRoles = {}
		expand = false
		open = true
	}

	export function openDrawerWithRef(ref: DataTableRef) {
		selectedSchemaKey = ref.schema
		selectedTableKey = ref.table
		selectDatatable(ref.datatable)
		selectedTables = []
		browsedRoles = {}
		expand = false
		open = true
	}

	export function closeDrawer() {
		open = false
		dbManagerContent?.clearReplResult()
		// An action outlives the data table it was asked for otherwise.
		pendingAction = undefined
	}

	function handleAddTables() {
		if (selectedTables.length === 0) {
			sendUserToast('Please select at least one table', true)
			return
		}
		recordBrowsedRole()

		const refs: DataTableRef[] = []
		for (const table of selectedTables) {
			const datatable = table.datatable ?? selectedDatatable
			if (!datatable) continue
			refs.push({ datatable, schema: table.schema, table: table.table })
		}
		const added = new Set(refs.map((r) => r.datatable))
		onAdd?.(refs, Object.fromEntries(Object.entries(browsedRoles).filter(([dt]) => added.has(dt))))

		const count = refs.length
		sendUserToast(`Added ${count} table${count > 1 ? 's' : ''} to app`)
		selectedTables = []
		browsedRoles = {}
	}

	// Carries the picked schema/table, so a click on a row of another data table lands on that
	// table once the manager re-mounts against it.
	const dbInput: DbInput | undefined = $derived(
		selectedDatatable
			? {
					type: 'database' as const,
					resourceType: 'postgresql' as const,
					resourcePath: `datatable://${selectedDatatable}`,
					role: selectedRole,
					specificSchema: openSchemaKey,
					specificTable: openTableKey
				}
			: undefined
	)

	$effect(() => {
		if (!open) {
			expand = false
		}
	})

	const disabledTables = $derived(
		existingRefs
			.filter((ref) => ref.schema && ref.table)
			.map((ref) => ({ datatable: ref.datatable, schema: ref.schema!, table: ref.table! }))
	)

	const canAdd = $derived(selectedTables.length > 0)

	// Shares the drawer-set override with the Database Manager: same data table,
	// same worker group needed to reach it.
	const workerTag = useDbManagerTag(
		() => opWs,
		() => dbInput
	)
</script>

<svelte:window bind:innerWidth={windowWidth} />

<Drawer bind:open size={expand ? `${windowWidth}px` : '1200px'} {offset}>
	<DrawerContent
		title="Data"
		on:close={() => {
			if (hasReplResult) {
				dbManagerContent?.clearReplResult()
			} else {
				closeDrawer()
			}
		}}
		CloseIcon={hasReplResult ? ArrowLeft : undefined}
		noPadding
	>
		{#if dbInput && opWs}
			{#if roleSettled}
				{#key `${selectedDatatable}~${selectedRole ?? ''}`}
					<DBManagerContent
						bind:this={dbManagerContent}
						input={dbInput}
						workspace={opWs}
						bind:workerTag={() => workerTag.tag, (v) => (workerTag.tag = v)}
						bind:hasReplResult
						bind:selectedSchemaKey
						bind:selectedTableKey
						multiSelectMode={true}
						bind:selectedTables
						{disabledTables}
						datatableTree={datatableTree.current}
						datatableTreeLoading={datatableTree.loading}
						onSelectDatatable={(dt) => selectDatatable(dt)}
						onSelectRole={(dt, role) => selectDatatable(dt, role)}
						bind:pendingAction
					/>
				{/key}
			{/if}
		{:else}
			<div class="flex items-center justify-center h-full text-tertiary">
				<span>Select a data table to explore</span>
			</div>
		{/if}

		{#snippet actions()}
			<Button
				variant="accent"
				disabled={!canAdd}
				on:click={handleAddTables}
				startIcon={{ icon: Plus }}
				unifiedSize="sm"
			>
				{#if selectedTables.length > 0}
					Add {selectedTables.length} table{selectedTables.length > 1 ? 's' : ''}
				{:else}
					Add to app
				{/if}
			</Button>

			{#if dbInput && opWs}
				<DbWorkerTagButton
					bind:tag={() => workerTag.tag, (v) => (workerTag.tag = v)}
					input={dbInput}
					workspace={opWs}
				/>
			{/if}

			<Button
				loading={dbManagerContent?.isLoading() ?? false}
				on:click={() => dbManagerContent?.refresh()}
				startIcon={{ icon: RefreshCcw }}
				unifiedSize="sm"
				variant="default"
				disabled={!selectedDatatable}
			>
				Refresh
			</Button>

			<Button
				on:click={() => (expand = !expand)}
				startIcon={{ icon: expand ? Minimize : Expand }}
				unifiedSize="sm"
				variant="default"
				iconOnly
			/>
		{/snippet}
	</DrawerContent>
</Drawer>
