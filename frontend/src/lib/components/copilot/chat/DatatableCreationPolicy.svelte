<script lang="ts">
	import { AlertTriangle } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import DefaultDatabaseSelector from '$lib/components/raw_apps/DefaultDatabaseSelector.svelte'
	import { workspaceStore } from '$lib/stores'
	import { createDatatablesResource } from '$lib/components/raw_apps/datatableUtils.svelte'

	// Load available datatables from workspace using shared utility
	const datatables = createDatatablesResource(() => $workspaceStore)

	const hasNoDatatables = $derived((datatables.current?.length ?? 0) === 0)

	// Auto-select first datatable when datatables load and none is selected
	$effect(() => {
		if (
			datatables.current &&
			datatables.current.length > 0 &&
			aiChatManager.datatableCreationPolicy.enabled &&
			!aiChatManager.datatableCreationPolicy.datatable
		) {
			aiChatManager.datatableCreationPolicy.datatable = datatables.current[0]
		}
	})

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

	function handleDefaultChange(datatable: string | undefined, schema: string | undefined) {
		aiChatManager.datatableCreationPolicy.datatable = datatable
		aiChatManager.datatableCreationPolicy.schema = schema
	}
</script>

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
		<DefaultDatabaseSelector
			datatable={aiChatManager.datatableCreationPolicy.datatable}
			schema={aiChatManager.datatableCreationPolicy.schema}
			onChange={handleDefaultChange}
			description="Set the default datatable and schema for new tables. When table creation is enabled, AI can create tables here if needed."
		/>
	{/if}
</div>