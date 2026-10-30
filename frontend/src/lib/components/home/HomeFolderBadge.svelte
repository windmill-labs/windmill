<!--
@component
What a folder or user row carries for the assistant: its instructions, skills and MCP servers,
counted on the row and named in a popover.
-->
<script lang="ts">
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { BookOpen, ExternalLink } from 'lucide-svelte'
	import { homeComposerContext } from '../sessions/homeComposerContext.svelte'
	import { scopeBadge, scopeOption, scopeRule } from '../copilot/chat/workingScope'

	let { root }: { root: string } = $props()

	const manager = $derived(homeComposerContext.runtime?.manager)
	const row = $derived(
		manager ? scopeOption(root, manager.scopeIdentity, manager.scopeResources) : undefined
	)
	const badge = $derived(row ? scopeBadge(row) : '')
</script>

{#if row && badge}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div onclick={(e) => e.stopPropagation()}>
		<Popover placement="bottom-end" contentClasses="p-3 w-72 flex flex-col gap-2 text-xs">
			{#snippet trigger()}
				<Button
					variant="subtle"
					unifiedSize="2xs"
					startIcon={{ icon: BookOpen }}
					nonCaptureEvent
				>
					{badge}
				</Button>
			{/snippet}
			{#snippet content()}
				{#if row.instructions.length}
					<div>
						<p class="text-2xs text-secondary">Instructions</p>
						{#each row.instructions as path (path)}
							<p class="font-mono text-primary">{path}</p>
						{/each}
					</div>
				{/if}
				{#if row.skills.length}
					<div>
						<p class="text-2xs text-secondary">Skills</p>
						{#each row.skills as path (path)}
							<p class="font-mono text-primary">{path}</p>
						{/each}
					</div>
				{/if}
				{#if row.mcpServers.length}
					<div>
						<p class="text-2xs text-secondary">MCP servers</p>
						{#each row.mcpServers as path (path)}
							<p class="font-mono text-primary">{path}</p>
						{/each}
					</div>
				{/if}
				<p class="text-2xs text-secondary">{scopeRule(root)}</p>
				<Button
					variant="subtle"
					unifiedSize="xs"
					endIcon={{ icon: ExternalLink }}
					href="/resources?owner={encodeURIComponent(root)}"
					btnClasses="!justify-start !px-0"
				>
					Open {root} in Resources
				</Button>
			{/snippet}
		</Popover>
	</div>
{/if}
