<!--
@component
What a folder or user row carries for the assistant: its instructions, skills and MCP servers,
one button each, counted. A click opens that section of Assistant settings on the folder.
-->
<script lang="ts">
	import { Button } from '$lib/components/common'
	import { BookOpen, Plug, ScrollText } from 'lucide-svelte'
	import { homeComposerContext } from '../sessions/homeComposerContext.svelte'
	import { scopeOption } from '../copilot/chat/workingScope'

	let { root }: { root: string } = $props()

	const manager = $derived(homeComposerContext.runtime?.manager)
	const row = $derived(
		manager ? scopeOption(root, manager.scopeIdentity, manager.scopeResources) : undefined
	)
	const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`
	const parts = $derived(
		row
			? [
					{
						section: 'instructions' as const,
						icon: ScrollText,
						count: row.instructions.length,
						label: plural(row.instructions.length, 'instruction')
					},
					{
						section: 'skills' as const,
						icon: BookOpen,
						count: row.skills.length,
						label: plural(row.skills.length, 'skill')
					},
					{
						section: 'mcp' as const,
						icon: Plug,
						count: row.mcpServers.length,
						label: `${row.mcpServers.length} MCP`
					}
				].filter((p) => p.count > 0)
			: []
	)
</script>

{#if parts.length}
	<span class="inline-flex items-center gap-0.5">
		{#each parts as part (part.section)}
			<Button
				variant="subtle"
				unifiedSize="2xs"
				startIcon={{ icon: part.icon }}
				title="Open in Assistant settings"
				onClick={(e) => {
					e?.stopPropagation()
					homeComposerContext.openSettings(part.section, root)
				}}
			>
				{part.label}
			</Button>
		{/each}
	</span>
{/if}
