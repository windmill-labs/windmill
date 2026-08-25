<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { DataTableRef } from './dataTableRefUtils'
	import { resource } from 'runed'
	import { ArrowLeft, Expand, Minimize, Plus, RefreshCcw } from 'lucide-svelte'
	import DBManagerContent from '../DBManagerContent.svelte'
	import type { DbInput } from '../dbTypes'
	import type { PendingCreate, SelectedTable } from '../DBManager.svelte'
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

	// For DB manager
	let dbManagerContent: DBManagerContent | undefined = $state()
	let hasReplResult = $state(false)
	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	// Multi-select mode: selected tables
	let selectedTables = $state<SelectedTable[]>([])

	// Survives the re-mount a data table switch causes.
	let pendingCreate = $state<PendingCreate | undefined>(undefined)

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

	// Every data table with its schemas and tables: the tree is the picker, so it
	// has to cover the data tables the query editor is not pointed at.
	const datatableTree = resource<DataTableTables[]>([], async () => {
		if (!opWs) return []
		try {
			return await WorkspaceService.listDataTableTables({ workspace: opWs })
		} catch (e) {
			console.error('Failed to load datatable tables:', e)
			return []
		}
	})

	export function openDrawer() {
		// The tree shows every data table; this only picks which one the query
		// editor and the table preview run against.
		selectedDatatable = datatables.current.includes('main') ? 'main' : datatables.current[0]
		selectedSchemaKey = undefined
		selectedTableKey = undefined
		selectedTables = []
		expand = false
		open = true
	}

	export function openDrawerWithRef(ref: DataTableRef) {
		selectedDatatable = ref.datatable
		selectedSchemaKey = ref.schema
		selectedTableKey = ref.table
		selectedTables = []
		expand = false
		open = true
	}

	export function closeDrawer() {
		open = false
		dbManagerContent?.clearReplResult()
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
			{#key selectedDatatable}
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
					onSelectDatatable={(dt) => (selectedDatatable = dt)}
					bind:pendingCreate
				/>
			{/key}
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
