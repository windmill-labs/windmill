<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { DataTableRef } from './dataTableRefUtils'
	import { untrack } from 'svelte'
	import { resource } from 'runed'
	import { ArrowLeft, Expand, Minimize, Plus, RefreshCcw } from 'lucide-svelte'
	import DBManagerContent from '../DBManagerContent.svelte'
	import type { DbInput } from '../dbTypes'
	import type { PendingRowAction, SelectedTable } from '../DBManager.svelte'
	import { getRawAppOperatingWorkspace } from './rawAppWorkspace'
	import { useDbManagerTag } from '../dbManagerTag.svelte'
	import DbWorkerTagButton from '../DbWorkerTagButton.svelte'
	import type { DataTableTables } from '$lib/gen'

	const getOpWs = getRawAppOperatingWorkspace()
	let opWs = $derived(getOpWs?.() ?? $workspaceStore)

	interface Props {
		onAdd?: (ref: DataTableRef) => void
		existingRefs?: DataTableRef[]
		/** Z-index offset for the drawer, useful when opening from within modals */
		offset?: number
	}

	let { onAdd, existingRefs = [], offset = 0 }: Props = $props()

	let open = $state(false)
	let selectedDatatable = $state<string | undefined>(undefined)
	/** Role the query editor connects as; undefined means the data table's default. */
	let selectedRole = $state<string | undefined>(undefined)

	// For DB manager
	let dbManagerContent: DBManagerContent | undefined = $state()
	let hasReplResult = $state(false)
	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	// Multi-select mode: selected tables
	let selectedTables = $state<SelectedTable[]>([])

	// Survives the re-mount a data table switch causes.
	let pendingAction = $state<PendingRowAction | undefined>(undefined)

	// Selected schema/table from DBManager (for preview)
	let selectedSchemaKey = $state<string | undefined>(undefined)
	let selectedTableKey = $state<string | undefined>(undefined)

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

	// Roles the *caller* may use, so the picker never offers one that would be
	// refused. Absent/disabled permissions yield no roles and hide the picker.
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
				// Never leave the drawer waiting on this: fall back to the
				// unpermissioned shape so it opens and the server picks the role.
				console.error('Failed to load datatable roles:', e)
				return { datatable, enabled: false, roles: [], default_role: 'admin' }
			}
		}
	)

	// Roles are per data table, and a resource keeps its previous value while it
	// refetches.
	const rolesOfCurrent = $derived(
		usableRoles.current?.datatable === selectedDatatable ? usableRoles.current : undefined
	)

	// The content must not mount until the role is settled: mounting is what fires
	// the schema and metadata queries, and a first round sent without a role would
	// run — and cache — as whatever the server defaults to.
	const roleSettled = $derived(
		selectedDatatable === undefined ||
			(rolesOfCurrent !== undefined &&
				(!rolesOfCurrent.enabled ||
					rolesOfCurrent.roles.length === 0 ||
					selectedRole !== undefined))
	)

	// Settle the role before anything queries the data table: leaving it unset
	// until the user touches the picker would send the first — and cached — round
	// of queries as a role they may not be allowed to use.
	$effect(() => {
		const roles = rolesOfCurrent
		if (!roles?.enabled || selectedRole !== undefined) return
		const effective = roles.roles.includes(roles.default_role) ? roles.default_role : roles.roles[0]
		if (effective) untrack(() => (selectedRole = effective))
	})

	// Every data table with its schemas and tables: the tree is the picker, so it
	// has to cover the data tables the query editor is not pointed at. Asked for
	// when the drawer opens — it reaches every data table's database in turn, and
	// the editor mounts this whether or not anyone opens it.
	const datatableTree = resource(
		// The privileges it reports are the connected role's, so the role picked on
		// the open data table is part of what is being asked — the same key its
		// sibling in the DB manager uses.
		() => [open, opWs, selectedDatatable, selectedRole] as const,
		async ([isOpen, workspace, roleFor, role]): Promise<DataTableTables[]> => {
			if (!isOpen || !workspace) return []
			try {
				return await WorkspaceService.listDataTableTables({ workspace, roleFor, role })
			} catch (e) {
				console.error('Failed to load datatable tables:', e)
				return []
			}
		},
		{ initialValue: [] }
	)

	export function openDrawer() {
		// The tree shows every data table; this only picks which one the query
		// editor and the table preview run against.
		selectedDatatable = datatables.current.includes('main') ? 'main' : datatables.current[0]
		// A role belongs to the data table it was picked on, and this drawer outlives
		// the session that picked it: kept, it would query another data table under a
		// role of that name, or under one it has never heard of.
		selectedRole = undefined
		selectedSchemaKey = undefined
		selectedTableKey = undefined
		selectedTables = []
		expand = false
		open = true
	}

	export function openDrawerWithRef(ref: DataTableRef) {
		selectedDatatable = ref.datatable
		selectedRole = undefined
		selectedSchemaKey = ref.schema
		selectedTableKey = ref.table
		selectedTables = []
		expand = false
		open = true
	}

	export function closeDrawer() {
		open = false
		dbManagerContent?.clearReplResult()
		// Same reason as its sibling: an action outlives the data table it was
		// asked for otherwise.
		pendingAction = undefined
	}

	function handleAddTables() {
		if (selectedTables.length === 0) {
			sendUserToast('Please select at least one table', true)
			return
		}

		for (const table of selectedTables) {
			const datatable = table.datatable ?? selectedDatatable
			if (!datatable) continue
			const ref: DataTableRef = {
				datatable,
				schema: table.schema,
				table: table.table
			}
			onAdd?.(ref)
		}

		const count = selectedTables.length
		sendUserToast(`Added ${count} table${count > 1 ? 's' : ''} to app`)
		selectedTables = []
	}

	// Carries the picked schema/table, so a click on a row of another data table
	// lands on that table once the manager re-mounts against it.
	const dbInput: DbInput | undefined = $derived(
		selectedDatatable
			? {
					type: 'database' as const,
					resourceType: 'postgresql' as const,
					resourcePath: `datatable://${selectedDatatable}`,
					role: selectedRole,
					specificSchema: selectedSchemaKey,
					specificTable: selectedTableKey
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
						onSelectDatatable={(dt) => ((selectedDatatable = dt), (selectedRole = undefined))}
						onSelectRole={(dt, role) => ((selectedDatatable = dt), (selectedRole = role))}
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
				variant="contained"
				color="blue"
				disabled={!canAdd}
				on:click={handleAddTables}
				startIcon={{ icon: Plus }}
				size="xs"
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
				size="xs"
				color="light"
				disabled={!selectedDatatable}
			>
				Refresh
			</Button>

			<Button
				on:click={() => (expand = !expand)}
				startIcon={{ icon: expand ? Minimize : Expand }}
				size="xs"
				color="light"
			/>
		{/snippet}
	</DrawerContent>
</Drawer>
