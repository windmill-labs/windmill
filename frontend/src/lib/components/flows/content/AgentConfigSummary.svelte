<script lang="ts">
	import { Badge, Section } from '$lib/components/common'
	import {
		AGENT_BRAIN_LABELS,
		summarizeAgentBrain,
		type AIAgentConfig
	} from '../agentResourceUtils'
	import { toolDisplayName, type AgentTool } from '../agentToolUtils'

	/** A saved agent's configuration, read-only and at a glance: what a detail page shows beside
	 *  running it, where the editor's form would be all fields and no summary. */
	interface Props {
		config: AIAgentConfig
	}

	let { config }: Props = $props()

	const SYSTEM_PROMPT_LABEL = AGENT_BRAIN_LABELS['system_prompt']

	let settings = $derived(
		summarizeAgentBrain(config).filter((r) => r.label !== SYSTEM_PROMPT_LABEL)
	)
	let systemPrompt = $derived(
		typeof config.system_prompt === 'string' && config.system_prompt !== ''
			? config.system_prompt
			: undefined
	)
	let tools = $derived(Array.isArray(config.tools) ? (config.tools as AgentTool[]) : [])

	function toolKind(tool: AgentTool): string {
		const value = tool?.value as Record<string, any> | undefined
		if (value?.tool_type === 'mcp') return 'MCP'
		if (value?.tool_type === 'websearch') return 'Web search'
		switch (value?.type) {
			case 'script':
				return 'Script'
			case 'flow':
				return 'Flow'
			case 'rawscript':
				return 'Inline script'
			case 'aiagent':
				return 'Agent'
			default:
				return 'Tool'
		}
	}
</script>

<div class="flex flex-col gap-6 p-4">
	<Section label="Model" small>
		<dl class="flex flex-col gap-2">
			{#each settings as row (row.label)}
				<div class="flex flex-col gap-0.5">
					<dt class="text-2xs text-secondary">{row.label}</dt>
					<dd class="text-xs text-primary break-words">{row.value}</dd>
				</div>
			{:else}
				<span class="text-xs text-secondary">Not configured</span>
			{/each}
		</dl>
	</Section>

	<Section label="System message" small>
		{#if systemPrompt}
			<p
				class="text-xs text-primary whitespace-pre-wrap break-words rounded-md bg-surface-secondary p-2 max-h-80 overflow-auto"
			>
				{systemPrompt}
			</p>
		{:else}
			<span class="text-xs text-secondary">None</span>
		{/if}
	</Section>

	<Section label="Tools ({tools.length})" small>
		{#if tools.length === 0}
			<span class="text-xs text-secondary">No tools</span>
		{:else}
			<ul class="flex flex-col gap-2">
				{#each tools as tool, i (tool?.id ?? i)}
					<li class="flex items-center justify-between gap-2 min-w-0">
						<span class="text-xs text-primary truncate">{toolDisplayName(tool) ?? tool?.id}</span>
						<Badge color="gray" small>{toolKind(tool)}</Badge>
					</li>
				{/each}
			</ul>
		{/if}
	</Section>
</div>
