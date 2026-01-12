<script lang="ts">
	import { Plus, Database, Trash2, Table2, Star } from 'lucide-svelte'
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { DataTableRef } from './dataTableRefUtils'
	import DefaultDatabaseSelector from './DefaultDatabaseSelector.svelte'
	import { Button } from '../common'

	// Re-export for backwards compatibility
	export type { DataTableRef } from './dataTableRefUtils'

	interface Props {
		dataTableRefs: DataTableRef[]
		/** Default datatable for new tables */
		defaultDatatable?: string | undefined
		/** Default schema for new tables */
		defaultSchema?: string | undefined
		onAdd?: () => void
		onRemove?: (index: number) => void
		onSelect?: (ref: DataTableRef, index: number) => void
		onDefaultChange?: (datatable: string | undefined, schema: string | undefined) => void
		selectedIndex?: number | undefined
		/** When true, renders without PanelSection wrapper (for use in modals) */
		standalone?: boolean
		/** Hide the default database selector */
		hideDefaultSelector?: boolean
	}

	let {
		dataTableRefs = [],
		defaultDatatable = undefined,
		defaultSchema = undefined,
		onAdd,
		onRemove,
		onSelect,
		onDefaultChange,
		selectedIndex = undefined,
		standalone = false,
		hideDefaultSelector = false
	}: Props = $props()

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

	// Sort entries with default datatable first
	const sortedDatatableEntries = $derived.by(() => {
		const entries = [...groupedRefs.entries()]
		return entries.sort(([a], [b]) => {
			if (a === defaultDatatable) return -1
			if (b === defaultDatatable) return 1
			return a.localeCompare(b)
		})
	})

	// Helper to sort schema entries with default schema first
	function sortSchemaEntries(
		entries: [string, { ref: DataTableRef; index: number }[]][],
		datatableName: string
	) {
		return entries.sort(([a], [b]) => {
			const aIsDefault =
				datatableName === defaultDatatable && (a === defaultSchema || (a === '' && !defaultSchema))
			const bIsDefault =
				datatableName === defaultDatatable && (b === defaultSchema || (b === '' && !defaultSchema))
			if (aIsDefault) return -1
			if (bIsDefault) return 1
			return a.localeCompare(b)
		})
	}
</script>

{#snippet actionButtons()}
	<div class="flex items-center gap-1">
		<!-- Settings popover for default database/schema -->
		{#if !hideDefaultSelector}
			<DefaultDatabaseSelector
				datatable={defaultDatatable}
				schema={defaultSchema}
				onChange={onDefaultChange}
			/>
		{/if}

		<!-- Add datatable button -->
		<Button
			onClick={() => onAdd?.()}
			title="Add datatable reference"
			unifiedSize="xs"
			variant="subtle"
			btnClasses="px-1 gap-0.5"
		>
			<Plus size={12} />
			<Database size={12} />
		</Button>
	</div>
{/snippet}

{#snippet tableList()}
	{#if dataTableRefs.length === 0}
		<span class="text-2xs text-tertiary">No tables referenced yet</span>
	{:else}
		<div class="flex flex-col w-full">
			{#each sortedDatatableEntries as [datatableName, schemaGroups] (datatableName)}
				{@const isDefaultDatatable = datatableName === defaultDatatable}
				<!-- Datatable header -->
				<div class="flex items-center gap-1.5 px-1 py-1 text-2xs text-tertiary">
					<Database size={12} class="shrink-0" />
					<span class="font-medium truncate">{datatableName}</span>
					{#if isDefaultDatatable}
						<span title="Default datatable">
							<Star size={10} class="shrink-0 text-primary" />
						</span>
					{/if}
				</div>

				{#each sortSchemaEntries([...schemaGroups.entries()], datatableName) as [schemaName, items] (schemaName)}
					{@const isDefaultSchema =
						isDefaultDatatable &&
						(schemaName === defaultSchema || (schemaName === '' && !defaultSchema))}
					<!-- Schema header (only if schema exists) -->
					{#if schemaName}
						<div class="flex items-center gap-1.5 pl-4 py-0.5 text-2xs text-tertiary">
							<span class="truncate">{schemaName}</span>
							{#if isDefaultSchema}
								<span title="Default schema">
									<Star size={8} class="shrink-0 text-primary" />
								</span>
							{/if}
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
{/snippet}

{#if standalone}
	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="text-sm text-primary">Existing tables to use</span>
			{@render actionButtons()}
		</div>
		{@render tableList()}
	</div>
{:else}
	<PanelSection
		fullHeight={false}
		size="sm"
		title="data"
		id="app-editor-data-panel"
		tooltip="Data tables to use in the app. Adding some here does not change the behavior of the app, since they can be used in code directly regardless. But it allows AI to always keep their schema in context as well as allowing quick access and clear view of the app's data layer."
	>
		{#snippet action()}
			{@render actionButtons()}
		{/snippet}

		{@render tableList()}
	</PanelSection>
{/if}
