<!--
@component
The Tools section of the assistant settings modal: every tool definition this chat
sends with each turn, listed by the model-facing name and description. Read-only —
tools are not individually switchable, they follow the mode and the connected servers.
-->
<script lang="ts">
	import { Section } from '$lib/components/common'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import type { ToolSummary } from './agentContext'

	let { tools }: { tools: ToolSummary[] } = $props()

	type Marked = ToolSummary & { marked?: string }

	let filter = $state('')
	// The same fuzzy search the home list and the pickers use, run twice: `marked` covers
	// whatever the haystack was, and the name and the description are two fields on the
	// row rather than one string, so each needs a pass of its own to be highlighted.
	let nameHits = $state<Marked[] | undefined>(undefined)
	let descriptionHits = $state<Marked[] | undefined>(undefined)

	let searching = $derived(filter.trim().length > 0)
	// Name matches first, then the tools that only matched on what they do. Within each
	// half the order is the one uFuzzy ranked.
	let rows: { tool: ToolSummary; name?: string; description?: string }[] = $derived.by(() => {
		if (!searching) return tools.map((tool) => ({ tool }))
		const byName = nameHits ?? []
		const named = new Set(byName.map((t) => t.name))
		return [
			...byName.map((t) => ({ tool: t, name: t.marked })),
			...(descriptionHits ?? [])
				.filter((t) => !named.has(t.name))
				.map((t) => ({ tool: t, description: t.marked }))
		]
	})
</script>

<SearchItems {filter} items={tools} bind:filteredItems={nameHits} f={(t) => t.name} />
<SearchItems
	{filter}
	items={tools}
	bind:filteredItems={descriptionHits}
	f={(t) => t.description}
/>

<Section
	label="Tools"
	description="What the assistant can call in this session: the built-in tools, plus whatever the connected MCP servers expose."
>
	<!-- Sticks to the top of the scrolling panel so a 70-row list stays searchable. -->
	<div class="sticky top-0 z-10 bg-surface pb-2">
		<TextInput bind:value={filter} size="sm" inputProps={{ placeholder: 'Search tools' }} />
	</div>
	{#each rows as row (row.tool.name)}
		<div class="py-2">
			<div class="font-mono text-xs text-emphasis break-all">
				{#if row.name}{@html row.name}{:else}{row.tool.name}{/if}
			</div>
			{#if row.tool.description}
				<div class="mt-0.5 text-xs text-secondary line-clamp-2">
					{#if row.description}{@html row.description}{:else}{row.tool.description}{/if}
				</div>
			{/if}
		</div>
	{/each}
	{#if rows.length === 0}
		<div class="py-2 text-xs text-hint">
			{tools.length === 0 ? 'This chat carries no tools.' : 'No tool matches this search.'}
		</div>
	{/if}
</Section>
