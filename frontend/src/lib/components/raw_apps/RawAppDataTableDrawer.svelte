<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Button from '../common/button/Button.svelte'
	import Select from '../select/Select.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { DataTableRef } from './RawAppDataTableList.svelte'
	import { resource } from 'runed'
	import { Loader2 } from 'lucide-svelte'

	interface Props {
		onAdd?: (ref: DataTableRef) => void
	}

	let { onAdd }: Props = $props()

	let open = $state(false)
	let selectedDatatable = $state<string | undefined>(undefined)
	let schema = $state<string | undefined>(undefined)
	let table = $state<string | undefined>(undefined)

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
		selectedDatatable = undefined
		schema = undefined
		table = undefined
		open = true
	}

	export function closeDrawer() {
		open = false
	}

	function handleAdd() {
		if (!selectedDatatable) {
			sendUserToast('Please select a data table', true)
			return
		}

		const ref: DataTableRef = {
			datatable: selectedDatatable,
			...(schema && { schema }),
			...(table && { table })
		}

		onAdd?.(ref)
		closeDrawer()
	}

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)
</script>

<Drawer bind:open size="400px">
	<DrawerContent title="Add Data Table Reference" on:close={closeDrawer}>
		<div class="flex flex-col gap-4">
			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-secondary">Data Table</span>
				<span class="text-xs text-tertiary">Select a data table from your workspace</span>
				{#if datatables.loading}
					<div class="flex items-center gap-2 p-2 text-tertiary">
						<Loader2 size={16} class="animate-spin" />
						<span class="text-sm">Loading datatables...</span>
					</div>
				{:else if datatables.current.length === 0}
					<div class="p-2 text-sm text-tertiary">
						No data tables configured in this workspace. Configure them in workspace settings.
					</div>
				{:else}
					<Select
						items={datatableItems}
						bind:value={selectedDatatable}
						placeholder="Select a data table"
					/>
				{/if}
			</label>

			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-secondary">Schema (optional)</span>
				<TextInput bind:value={schema} inputProps={{ placeholder: 'e.g., public' }} />
			</label>

			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-secondary">Table</span>
				<TextInput bind:value={table} inputProps={{ placeholder: 'e.g., users' }} />
			</label>
		</div>

		{#snippet actions()}
			<Button variant="border" color="light" on:click={closeDrawer}>Cancel</Button>
			<Button variant="contained" color="blue" disabled={!selectedDatatable} on:click={handleAdd}>
				Add
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
