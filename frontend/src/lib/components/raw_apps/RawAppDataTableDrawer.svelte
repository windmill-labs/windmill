<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import Select from '../select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { DataTableRef } from './RawAppDataTableList.svelte'
	import { resource } from 'runed'
	import { ArrowLeft, Expand, LoaderCircle, Minimize, Plus, RefreshCcw } from 'lucide-svelte'
	import DBManagerContent from '../DBManagerContent.svelte'
	import type { DbInput } from '../dbTypes'

	interface Props {
		onAdd?: (ref: DataTableRef) => void
		existingRefs?: DataTableRef[]
	}

	let { onAdd, existingRefs = [] }: Props = $props()

	let open = $state(false)
	let selectedDatatable = $state<string | undefined>(undefined)

	// For DB manager
	let dbManagerContent: DBManagerContent | undefined = $state()
	let hasReplResult = $state(false)
	let isRefreshing = $state(false)
	let windowWidth = $state(window.innerWidth)
	let expand = $state(false)

	// Selected schema/table from DBManager
	let selectedSchemaKey = $state<string | undefined>(undefined)
	let selectedTableKey = $state<string | undefined>(undefined)

	// Load available datatables from workspace
	const datatables = resource<string[]>([], async () => {
		if (!$workspaceStore) return []
		try {
			return await WorkspaceService.listDataTables({ workspace: $workspaceStore })
		} catch (e) {
			console.error('Failed to load datatables:', e)
			return []
		}
	})

	export function openDrawer() {
		// Auto-select first datatable if only one exists
		if (datatables.current.length === 1) {
			selectedDatatable = datatables.current[0]
		} else if (datatables.current.length > 1 && datatables.current.includes('main')) {
			selectedDatatable = 'main'
		} else {
			selectedDatatable = undefined
		}
		selectedSchemaKey = undefined
		selectedTableKey = undefined
		expand = false
		open = true
	}

	let initialTableKey: string | undefined = $state<string | undefined>(undefined)
	let initialSchemaKey: string | undefined = $state<string | undefined>(undefined)

	export function openDrawerWithRef(ref: DataTableRef) {
		selectedDatatable = ref.datatable
		selectedSchemaKey = ref.schema
		selectedTableKey = ref.table
		initialTableKey = ref.table
		initialSchemaKey = ref.schema
		expand = false
		open = true
	}

	export function closeDrawer() {
		open = false
		dbManagerContent?.clearReplResult()
	}

	function handleAddTable() {
		if (!selectedDatatable) {
			sendUserToast('Please select a data table first', true)
			return
		}

		const ref: DataTableRef = {
			datatable: selectedDatatable,
			...(selectedSchemaKey && { schema: selectedSchemaKey }),
			...(selectedTableKey && { table: selectedTableKey })
		}

		onAdd?.(ref)
		sendUserToast(`Added ${selectedTableKey ?? selectedSchemaKey ?? selectedDatatable} to app`)
	}

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	const dbInput: DbInput | undefined = $derived(
		selectedDatatable
			? {
					type: 'database' as const,
					resourceType: 'postgresql' as const,
					resourcePath: `datatable://${selectedDatatable}`,
					specificSchema: initialSchemaKey,
					specificTable: initialTableKey
				}
			: undefined
	)

	$effect(() => {
		if (!open) {
			expand = false
		}
	})

	// Check if current selection is already in the list
	const isCurrentSelectionInList = $derived.by(() => {
		if (!selectedDatatable) return false
		return existingRefs.some(
			(ref) =>
				ref.datatable === selectedDatatable &&
				ref.schema === selectedSchemaKey &&
				ref.table === selectedTableKey
		)
	})

	// Can add: has a table selected and it's not already in the list
	const canAdd = $derived(selectedDatatable && selectedTableKey && !isCurrentSelectionInList)
</script>

<svelte:window bind:innerWidth={windowWidth} />

<Drawer bind:open size={expand ? `${windowWidth}px` : '1200px'}>
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
		{#if dbInput && $workspaceStore}
			{#key selectedDatatable}
				<DBManagerContent
					bind:this={dbManagerContent}
					input={dbInput}
					bind:hasReplResult
					bind:isRefreshing
					bind:selectedSchemaKey
					bind:selectedTableKey
				>
					{#snippet dbSelector()}
						{#if datatables.loading}
							<div class="flex items-center gap-2 text-tertiary ml-2">
								<LoaderCircle size={14} class="animate-spin" />
								<span class="text-sm">Loading...</span>
							</div>
						{:else if datatables.current.length >= 1}
							<Select
								transformInputSelectedText={(s) => `Datatable: ${s}`}
								items={datatableItems}
								bind:value={selectedDatatable}
								placeholder="Select data table"
								size="md"
							/>
						{/if}
					{/snippet}
				</DBManagerContent>
			{/key}
		{:else}
			<div class="flex items-center justify-center h-full text-tertiary">
				<span>Select a data table to explore</span>
			</div>
		{/if}

		{#snippet actions()}
			{#if selectedTableKey}
				<Button
					variant="contained"
					color="blue"
					disabled={!canAdd}
					on:click={handleAddTable}
					startIcon={{ icon: Plus }}
					size="xs"
				>
					{isCurrentSelectionInList ? 'Already added' : 'Add to app'}
				</Button>
			{/if}

			<Button
				loading={isRefreshing}
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
