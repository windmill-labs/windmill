<script lang="ts">
	import { Settings, AlertTriangle } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import { workspaceStore, dbSchemas } from '$lib/stores'
	import { WorkspaceService } from '$lib/gen'
	import { resource } from 'runed'
	import { getDbSchemas } from '$lib/components/apps/components/display/dbtable/metadata'
	import Select from '$lib/components/select/Select.svelte'

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

	const hasNoDatatables = $derived((datatables.current?.length ?? 0) === 0)

	// Auto-select first datatable when datatables load and none is selected
	$effect(() => {
		if (
			datatables.current.length > 0 &&
			aiChatManager.datatableCreationPolicy.enabled &&
			!aiChatManager.datatableCreationPolicy.datatable
		) {
			aiChatManager.datatableCreationPolicy.datatable = datatables.current[0]
		}
	})

	// Load schemas for the selected datatable
	const selectedDatatable = $derived(aiChatManager.datatableCreationPolicy.datatable)
	const schemas = resource<string[]>([], async () => {
		if (!selectedDatatable || !$workspaceStore) return []

		const resourcePath = `datatable://${selectedDatatable}`
		let schema = $dbSchemas[resourcePath]

		if (!schema) {
			try {
				await getDbSchemas('postgresql', resourcePath, $workspaceStore, $dbSchemas, (msg) =>
					console.error('Schema error:', msg)
				)
				schema = $dbSchemas[resourcePath]
			} catch (e) {
				console.error(`Failed to load schema for ${selectedDatatable}:`, e)
				return []
			}
		}

		if (!schema?.schema) return []
		return Object.keys(schema.schema)
	})

	const datatableItems = $derived(
		datatables.current.map((dt) => ({
			value: dt,
			label: dt
		}))
	)

	const schemaItems = $derived([
		...schemas.current.map((s) => ({
			value: s,
			label: s
		}))
	])

	function handleToggle(enabled: boolean) {
		aiChatManager.datatableCreationPolicy.enabled = enabled
		if (
			enabled &&
			datatables.current.length > 0 &&
			!aiChatManager.datatableCreationPolicy.datatable
		) {
			aiChatManager.datatableCreationPolicy.datatable = datatables.current[0]
		}
	}

	// Track datatable changes to reset schema
	let previousDatatable = $state<string | undefined>(undefined)
	$effect(() => {
		const currentDatatable = aiChatManager.datatableCreationPolicy.datatable
		if (previousDatatable !== undefined && currentDatatable !== previousDatatable) {
			// Reset schema when datatable changes
			aiChatManager.datatableCreationPolicy.schema = undefined
		}
		previousDatatable = currentDatatable
	})
</script>

{#if aiChatManager.mode === AIMode.APP}
	<div class="min-w-0 flex items-center gap-1 pt-0.5">
		{#if hasNoDatatables}
			<!-- Warning when no datatables are available -->
			<div
				class="text-2xs flex flex-row items-center gap-1 text-red-600 dark:text-red-400 px-1"
				title="No datatables configured. Add datatables in the Data panel so AI can create tables."
			>
				<AlertTriangle size={12} class="shrink-0" />
				<span class="truncate">No datatables</span>
			</div>
		{:else}
			<!-- Toggle for new tables -->
			<div class="flex items-center gap-1">
				<Toggle
					size="xs"
					checked={aiChatManager.datatableCreationPolicy.enabled}
					on:change={(e) => handleToggle(e.detail)}
				/>
				<span class="text-2xs text-secondary whitespace-nowrap">tables creation</span>
			</div>

			<!-- Settings icon with popover -->
			<Popover>
				<svelte:fragment slot="trigger">
					<button
						class="p-0.5 hover:bg-surface-hover rounded transition-colors"
						title="Configure table creation settings"
					>
						<Settings size={12} class="text-tertiary" />
					</button>
				</svelte:fragment>
				<svelte:fragment slot="content">
					<div class="flex flex-col gap-3 p-2 min-w-64 max-w-80">
						<div class="text-xs font-medium text-primary">Table Creation Settings</div>

						<!-- Explanation -->
						<p class="text-2xs text-tertiary leading-relaxed">
							When enabled, AI knows it can create new tables if the existing tables in the Data
							panel are not sufficient for storing the app's data. Tables will be created in the
							selected datatable and schema.
						</p>

						<!-- Datatable selector -->
						<div class="flex flex-col gap-1">
							<span class="text-2xs text-tertiary">Datatable</span>
							<Select
								items={datatableItems}
								bind:value={aiChatManager.datatableCreationPolicy.datatable}
								placeholder="Select datatable"
								size="sm"
							/>
						</div>

						<!-- Schema selector -->
						<div class="flex flex-col gap-1">
							<span class="text-2xs text-tertiary">Schema</span>
							<Select
								items={schemaItems}
								bind:value={
									() => aiChatManager.datatableCreationPolicy.schema ?? '',
									(v) => (aiChatManager.datatableCreationPolicy.schema = v || undefined)
								}
								placeholder="Any schema"
								size="sm"
							/>
						</div>
					</div>
				</svelte:fragment>
			</Popover>
		{/if}
	</div>
{/if}
