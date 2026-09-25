<!--
@component
A saved agent's configuration, read-only: the model it calls, the instructions it is given and
the tools it can use. Laid out as the AI session's assistant settings are, and its Tools section
is that one, since both answer the same question about what an assistant can see and use.
-->
<script lang="ts">
	import { Boxes, Cpu, ScrollText } from 'lucide-svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'
	import AssistantToolsSection from '$lib/components/copilot/chat/AssistantToolsSection.svelte'
	import type { ToolSummary } from '$lib/components/copilot/chat/agentContext'
	import {
		AGENT_BRAIN_LABELS,
		summarizeAgentBrain,
		type AIAgentConfig
	} from '../agentResourceUtils'
	import { toolDisplayName, type AgentTool } from '../agentToolUtils'

	type Section = 'model' | 'instructions' | 'tools'

	interface Props {
		config: AIAgentConfig
		/** The arguments a tool takes, where they are known. */
		toolSchema?: (id: string) => any
	}

	let { config, toolSchema = undefined }: Props = $props()

	let isOpen = $state(false)
	let section = $state<Section>('model')
	let toolsBusy = $state(false)

	export function open(target: Section = section) {
		section = target
		isOpen = true
	}

	const SYSTEM_PROMPT_LABEL = AGENT_BRAIN_LABELS['system_prompt']

	let settings = $derived(
		summarizeAgentBrain(config).filter((r) => r.label !== SYSTEM_PROMPT_LABEL)
	)
	let systemPrompt = $derived(
		typeof config.system_prompt === 'string' && config.system_prompt !== ''
			? config.system_prompt
			: undefined
	)
	let tools = $derived<ToolSummary[]>(
		(Array.isArray(config.tools) ? (config.tools as AgentTool[]) : [])
			.map((tool) => ({
				name: toolDisplayName(tool) ?? tool?.id ?? '',
				description: toolKind(tool),
				parameters: { required: [], ...(toolSchema?.(tool?.id) ?? {}) }
			}))
			.sort((a, b) => a.name.localeCompare(b.name))
			// The section keys and deduplicates its rows by name, and two MCP servers or web searches
			// can share one, so a repeat is told apart by its position.
			.map((tool, i, all) => {
				const nth = all.slice(0, i).filter((t) => t.name === tool.name).length
				return nth === 0 ? tool : { ...tool, name: `${tool.name} (${nth + 1})` }
			})
	)

	let sections = $derived([
		{ id: 'model', label: 'Model', icon: Cpu },
		{ id: 'instructions', label: 'Instructions', icon: ScrollText },
		{ id: 'tools', label: 'Tools', icon: Boxes, count: tools.length }
	])

	function toolKind(tool: AgentTool): string {
		const value = tool?.value as Record<string, any> | undefined
		if (value?.tool_type === 'mcp') return 'MCP server'
		if (value?.tool_type === 'websearch') return 'Web search'
		switch (value?.type) {
			case 'script':
				return `Script ${value.path ?? ''}`.trim()
			case 'flow':
				return `Flow ${value.path ?? ''}`.trim()
			case 'rawscript':
				return 'Inline script'
			case 'aiagent':
				return 'Agent'
			default:
				return 'Tool'
		}
	}
</script>

<Modal2
	bind:isOpen
	title="Agent configuration"
	fixedWidth="md"
	fixedHeight="lg"
	closeOnOutsideClick={!toolsBusy}
	closeOnEscape={!toolsBusy}
>
	{#snippet headerLeft()}
		<p class="pl-3 pt-1 text-xs text-secondary truncate">
			What the agent runs with. Edit the agent to change it.
		</p>
	{/snippet}

	<div class="w-full flex min-h-0 gap-4">
		<div class="w-52 shrink-0 flex flex-col border-r border-border-light pr-3">
			<SidebarNavigation
				groups={[{ items: sections }]}
				selectedId={section}
				onNavigate={(id) => (section = id as Section)}
			/>
		</div>

		<div class="grow min-w-0 flex flex-col min-h-0">
			{#if section === 'model'}
				<dl class="flex flex-col gap-3 overflow-y-auto pr-2">
					{#each settings as row (row.label)}
						<div class="flex flex-col gap-0.5">
							<dt class="text-2xs text-secondary">{row.label}</dt>
							<dd class="text-xs text-primary break-words">{row.value}</dd>
						</div>
					{:else}
						<span class="text-xs text-secondary">Not configured</span>
					{/each}
				</dl>
			{:else if section === 'instructions'}
				<div class="overflow-y-auto pr-2">
					{#if systemPrompt}
						<p
							class="text-xs text-primary whitespace-pre-wrap break-words rounded-md bg-surface-secondary p-3"
						>
							{systemPrompt}
						</p>
					{:else}
						<span class="text-xs text-secondary">No system message</span>
					{/if}
				</div>
			{/if}
			<!-- Mounted throughout, as the assistant settings keep it: it owns its scrolling and
			     the detail page it opens. -->
			<div class="{section === 'tools' ? 'flex' : 'hidden'} grow min-h-0 flex-col overflow-hidden">
				<AssistantToolsSection
					{tools}
					description="What the agent can call: scripts, flows, other agents, MCP servers and web search."
					active={section === 'tools'}
					bind:blocksClose={toolsBusy}
				/>
			</div>
		</div>
	</div>
</Modal2>
