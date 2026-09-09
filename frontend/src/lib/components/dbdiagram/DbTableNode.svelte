<script lang="ts">
	import { ChevronDown, ChevronUp, KeyRound, Link2, Table2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { columnKeyIn, type RelationIndex } from '../dbRelations'
	import { getDbDiagramHighlight } from './dbDiagramHighlight.svelte'
	import {
		CARD_WIDTH,
		HEADER_HEIGHT,
		ROW_HEIGHT,
		hasExpandToggle,
		hiddenColumnCount,
		visibleColumns,
		type DiagramTable
	} from './dbDiagramModel'

	interface Props {
		data: {
			table: DiagramTable
			expanded: boolean
			index: RelationIndex
			onToggleExpand: () => void
			onOpenTable: () => void
		}
	}

	let { data }: Props = $props()

	const highlight = getDbDiagramHighlight()

	let table = $derived(data.table)
	let columns = $derived(visibleColumns(table, data.expanded))
	let hidden = $derived(hiddenColumnCount(table, data.expanded))

	let isTarget = $derived(highlight.target?.table === table.key)
	let lit = $derived(highlight.tables?.has(table.key) ?? true)
	let dimmed = $derived(!!highlight.tables && !lit)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'rounded-md border bg-surface-tertiary shadow-sm transition-opacity duration-150 overflow-hidden',
		dimmed ? 'opacity-20' : 'opacity-100',
		isTarget
			? 'border-border-accent'
			: lit && highlight.tables
				? 'border-border-selected'
				: 'border-border-light'
	)}
	style="width: {CARD_WIDTH}px"
	onmouseenter={() => highlight.hover({ table: table.key })}
	onmouseleave={() => highlight.hover(undefined)}
	onclick={() => highlight.togglePin({ table: table.key })}
	ondblclick={data.onOpenTable}
	role="button"
	tabindex="-1"
>
	<div
		class={twMerge(
			'flex items-center gap-1.5 px-2 border-b border-border-light',
			isTarget ? 'bg-surface-accent-selected' : 'bg-surface-secondary'
		)}
		style="height: {HEADER_HEIGHT}px"
		title={table.key}
	>
		<Table2 size={13} class="shrink-0 text-secondary" />
		<span class="truncate text-xs font-semibold text-emphasis">{table.table}</span>
		<span class="truncate text-2xs text-tertiary ml-auto">{table.schema}</span>
	</div>

	{#each columns as column (column.name)}
		{@const columnKey = columnKeyIn(table.key, column.name)}
		{@const isLit = highlight.columns?.has(columnKey)}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class={twMerge(
				'flex items-center gap-1.5 px-2 text-2xs',
				isLit ? 'bg-surface-accent-selected text-accent font-medium' : 'text-primary'
			)}
			style="height: {ROW_HEIGHT}px"
			onmouseenter={() => highlight.hover({ table: table.key, column: column.name })}
			onmouseleave={() => highlight.hover({ table: table.key })}
			onclick={(e) => {
				e.stopPropagation()
				highlight.togglePin({ table: table.key, column: column.name })
			}}
		>
			{#if column.isPrimaryKey}
				<KeyRound size={11} class="shrink-0 text-yellow-500" />
			{:else if data.index.referencingColumns.has(columnKey)}
				<Link2 size={11} class="shrink-0 text-secondary" />
			{:else}
				<span class="shrink-0 w-[11px]"></span>
			{/if}
			<span class="truncate">{column.name}</span>
			<span class="truncate text-hint ml-auto pl-2">{column.datatype}</span>
		</div>
	{/each}

	{#if hasExpandToggle(table)}
		<button
			class="flex items-center gap-1 px-2 w-full text-2xs text-tertiary hover:bg-surface-hover"
			style="height: {ROW_HEIGHT}px"
			onclick={(e) => {
				e.stopPropagation()
				data.onToggleExpand()
			}}
		>
			{#if data.expanded}
				<ChevronUp size={11} class="shrink-0" />
				<span>Show less</span>
			{:else}
				<ChevronDown size={11} class="shrink-0" />
				<span>{hidden} more</span>
			{/if}
		</button>
	{/if}
</div>
