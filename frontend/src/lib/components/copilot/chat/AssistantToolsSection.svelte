<!--
@component
The Tools section of the assistant settings modal: every tool definition this chat
sends with each turn, listed by the model-facing name and description. Read-only —
tools are not individually switchable, they follow the mode and the connected servers.
-->
<script lang="ts">
	import { Section } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { filterTools, type ToolSummary } from './agentContext'

	let { tools }: { tools: ToolSummary[] } = $props()

	let filter = $state('')
	let shown = $derived(filterTools(tools, filter))
</script>

<Section
	label="Tools"
	description="What the assistant can call in this session: the built-in tools, plus whatever the connected MCP servers expose."
>
	<!-- Sticks to the top of the scrolling panel so a 70-row list stays searchable. -->
	<div class="sticky top-0 z-10 bg-surface pb-2">
		<TextInput bind:value={filter} size="sm" inputProps={{ placeholder: 'Search tools' }} />
	</div>
	{#each shown as tool (tool.name)}
		<div class="py-2">
			<div class="font-mono text-xs text-emphasis break-all">{tool.name}</div>
			{#if tool.description}
				<div class="mt-0.5 text-xs text-secondary line-clamp-2">{tool.description}</div>
			{/if}
		</div>
	{/each}
	{#if shown.length === 0}
		<div class="py-2 text-xs text-hint">
			{tools.length === 0 ? 'This chat carries no tools.' : 'No tool matches this search.'}
		</div>
	{/if}
</Section>
