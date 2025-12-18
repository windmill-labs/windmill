<script lang="ts">
	import { Plus, Database, Trash2 } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'

	export interface DataTableRef {
		/** The datatable name from workspace settings */
		datatable: string
		/** Optional schema filter */
		schema?: string
		/** Optional table filter */
		table?: string
	}

	interface Props {
		dataTableRefs: DataTableRef[]
		onAdd?: () => void
		onRemove?: (index: number) => void
		onSelect?: (ref: DataTableRef, index: number) => void
		selectedIndex?: number | undefined
	}

	let {
		dataTableRefs = $bindable([]),
		onAdd,
		onRemove,
		onSelect,
		selectedIndex = undefined
	}: Props = $props()
</script>

<PanelSection fullHeight={false} size="lg" title="data" id="app-editor-data-panel">
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
		<div class="flex flex-col gap-1 w-full">
			{#each dataTableRefs as ref, index (index)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="group flex items-center gap-2 px-2 py-1.5 rounded text-left w-full hover:bg-surface-hover transition-colors cursor-pointer {selectedIndex ===
					index
						? 'bg-surface-selected'
						: ''}"
					onclick={() => onSelect?.(ref, index)}
				>
					<Database size={14} class="text-tertiary shrink-0" />
					<div class="flex flex-col min-w-0 flex-1">
						<span class="text-2xs text-tertiary truncate">
							{ref.datatable}{#if ref.schema}.{ref.schema}{/if}{#if ref.table}.{ref.table}{/if}
						</span>
					</div>
					<button
						class="p-1 hover:bg-red-500/10 rounded transition-colors opacity-0 group-hover:opacity-100"
						onclick={(e) => {
							e.stopPropagation()
							onRemove?.(index)
						}}
						title="Remove data table reference"
					>
						<Trash2 size={12} class="text-red-500" />
					</button>
				</div>
			{/each}
		</div>
	{/if}
</PanelSection>
