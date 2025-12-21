<script lang="ts">
	import { Plus, Database, Trash2, Table2 } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { DataTableRef } from './dataTableRefUtils'

	// Re-export for backwards compatibility
	export type { DataTableRef } from './dataTableRefUtils'

	interface Props {
		dataTableRefs: DataTableRef[]
		onAdd?: () => void
		onRemove?: (index: number) => void
		onSelect?: (ref: DataTableRef, index: number) => void
		selectedIndex?: number | undefined
	}

	let { dataTableRefs = [], onAdd, onRemove, onSelect, selectedIndex = undefined }: Props = $props()

	// Group refs by datatable, then by schema
	type GroupedRefs = Map<string, Map<string, { ref: DataTableRef; index: number }[]>>

	const groupedRefs = $derived.by(() => {
		const groups: GroupedRefs = new Map()
		dataTableRefs.forEach((ref, index) => {
			const datatableKey = ref.datatable
			const schemaKey = ref.schema ?? ''

			if (!groups.has(datatableKey)) {
				groups.set(datatableKey, new Map())
			}
			const datatableGroup = groups.get(datatableKey)!
			if (!datatableGroup.has(schemaKey)) {
				datatableGroup.set(schemaKey, [])
			}
			datatableGroup.get(schemaKey)!.push({ ref, index })
		})
		return groups
	})
</script>

<PanelSection
	fullHeight={false}
	size="lg"
	title="data"
	id="app-editor-data-panel"
	tooltip="Data tables to use in the app. Adding some here does not change the behavior of the app, since they can be used in code directly regardless. But it allows AI to always keep their schema in context as well as allowing quick access and clear view of the app's data layer."
>
	{#snippet action()}
		<button
			onclick={() => onAdd?.()}
			class="p-0.5 hover:bg-surface-hover rounded transition-colors flex items-center gap-0.5"
			title="Add data table reference"
		>
			<Plus size={12} class="text-secondary" />
			<Database size={12} class="text-tertiary" />
		</button>
	{/snippet}

	{#if dataTableRefs.length === 0}
		<span class="text-2xs text-tertiary">No data tables configured</span>
	{:else}
		<div class="flex flex-col w-full">
			{#each [...groupedRefs.entries()] as [datatableName, schemaGroups] (datatableName)}
				<!-- Datatable header -->
				<div class="flex items-center gap-1.5 px-1 py-1 text-2xs text-tertiary">
					<Database size={12} class="shrink-0" />
					<span class="font-medium truncate">{datatableName}</span>
				</div>

				{#each [...schemaGroups.entries()] as [schemaName, items] (schemaName)}
					<!-- Schema header (only if schema exists) -->
					{#if schemaName}
						<div class="flex items-center gap-1.5 pl-4 py-0.5 text-2xs text-tertiary">
							<span class="truncate">{schemaName}</span>
						</div>
					{/if}

					<!-- Tables -->
					{#each items as { ref, index } (index)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="group flex items-center gap-2 py-1 rounded text-left w-full hover:bg-surface-hover transition-colors cursor-pointer {selectedIndex ===
							index
								? 'bg-surface-selected'
								: ''}"
							class:pl-7={schemaName}
							class:pl-4={!schemaName}
							onclick={() => onSelect?.(ref, index)}
						>
							<Table2 size={12} class="text-tertiary shrink-0" />
							<span class="text-2xs text-secondary truncate flex-1">
								{ref.table ?? '(all tables)'}
							</span>
							<button
								class="p-1 hover:bg-red-500/10 rounded transition-colors opacity-0 group-hover:opacity-100 mr-1"
								onclick={(e) => {
									e.stopPropagation()
									onRemove?.(index)
								}}
								title="Remove table reference"
							>
								<Trash2 size={10} class="text-red-500" />
							</button>
						</div>
					{/each}
				{/each}
			{/each}
		</div>
	{/if}
</PanelSection>
