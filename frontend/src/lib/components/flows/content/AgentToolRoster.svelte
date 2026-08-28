<script lang="ts">
	import { ChevronRight, Plus, Wrench } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { getToolNameError, toolDisplayName } from '../agentToolUtils'
	import type { AgentTool } from '../agentToolUtils'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import InsertModuleInner from '../map/InsertModuleInner.svelte'
	import { overlayPortalTarget } from '$lib/components/common/overlayHost.svelte'

	interface Props {
		tools?: AgentTool[]
		/** Where clicking a tool goes. Without it the roster is a read-only list. */
		onSelectTool?: (toolId: string) => void
		/** Adds a tool. Without it the empty state only says where to add one. */
		onAddTool?: (detail: { kind: string; script?: any; flow?: any; inlineScript?: any }) => void
		emptyMessage?: string
		/** Where the picker's popover belongs, when the roster is not inside the flow editor. */
		pickerPortal?: string
	}

	let {
		tools = [],
		onSelectTool = undefined,
		onAddTool = undefined,
		emptyMessage = 'No tools yet. Add one from the agent on the flow graph.',
		pickerPortal = '#flow-editor'
	}: Props = $props()

	let funcDesc = $state('')
	const portalTarget = overlayPortalTarget(() => pickerPortal)

	function toolKind(tool: AgentTool): string | undefined {
		const value = tool.value as Record<string, any>
		if (value?.type === 'aiagent') return 'agent'
		if (value?.tool_type === 'mcp') return 'MCP'
		if (value?.tool_type === 'websearch') return 'web search'
		return undefined
	}

	/** Only a flow module tool's summary is the name the model is given; an mcp or websearch
	 *  summary is never sent, so a blank one there is not an error. Same rule as `agentToolTree`. */
	function isNamed(tool: AgentTool): boolean {
		return tool.value?.tool_type !== 'mcp' && tool.value?.tool_type !== 'websearch'
	}

	let siblingNames = $derived(tools.filter(isNamed).map((tool) => tool.summary ?? ''))

	function nameError(tool: AgentTool): string | undefined {
		if (!isNamed(tool)) return undefined
		return getToolNameError(tool.summary ?? '', tool.value?.tool_type, siblingNames)
	}
</script>

{#snippet addToolButton()}
	<Popover
		portal={portalTarget()}
		contentClasses="p-2 max-w-full h-[400px] bg-surface"
		class="inline-block"
		usePointerDownOutside
		floatingConfig={{ placement: 'bottom-start', strategy: 'absolute', gutter: 8, flip: true }}
	>
		{#snippet trigger()}
			<Button variant="default" unifiedSize="sm" startIcon={{ icon: Plus }}>Tool</Button>
		{/snippet}
		{#snippet content({ close })}
			<!-- The same picker the graph's `+ Tool` opens, so both routes offer the same kinds and
			     hand the same detail to the same insert. -->
			<InsertModuleInner
				bind:funcDesc
				toolMode
				on:close={close}
				on:new={(e) => (onAddTool?.(e.detail), close())}
				on:insert={(e) => (onAddTool?.(e.detail), close())}
				on:pickScript={(e) => (
					onAddTool?.({
						kind: e.detail.kind,
						script: {
							...e.detail,
							summary: e.detail.summary
								? e.detail.summary.replace(/\s/, '_').replace(/[^a-zA-Z0-9_]/g, '')
								: e.detail.path.split('/').pop()
						}
					}),
					close()
				)}
				on:pickMcpTool={() => (onAddTool?.({ kind: 'mcpTool' }), close())}
				on:pickWebsearchTool={() => (onAddTool?.({ kind: 'websearchTool' }), close())}
				on:pickAiAgentTool={() => (onAddTool?.({ kind: 'aiAgentTool' }), close())}
			/>
		{/snippet}
	</Popover>
{/snippet}

{#if tools.length === 0}
	<div class="flex flex-col items-start gap-2">
		<div class="text-xs text-tertiary">
			{onAddTool ? 'No tools yet.' : emptyMessage}
		</div>
		{#if onAddTool}
			{@render addToolButton()}
		{/if}
	</div>
{:else}
	<div class="flex flex-col border rounded-md divide-y overflow-hidden">
		{#each tools as tool (tool.id)}
			{@const kind = toolKind(tool)}
			{@const error = nameError(tool)}
			<button
				type="button"
				disabled={!onSelectTool}
				onclick={() => onSelectTool?.(tool.id)}
				class="flex flex-row items-center gap-2 px-2 py-1.5 text-left text-xs text-primary
					enabled:hover:bg-surface-hover disabled:cursor-default"
			>
				<Wrench size={13} class="shrink-0 text-tertiary" />
				<!-- A tool the worker will reject is named by its problem, not by an id no one chose:
				     the id would read as a name and hide that the run cannot start. -->
				<span class={twMerge('truncate shrink min-w-0', error && 'text-red-400')}>
					{error ? tool.summary || 'Missing name' : toolDisplayName(tool)}
				</span>
				{#if error && tool.summary}
					<span class="truncate grow min-w-0 text-2xs text-red-400">{error}</span>
				{:else}
					<span class="grow"></span>
				{/if}
				{#if kind}
					<span class="text-2xs text-tertiary shrink-0">{kind}</span>
				{/if}
				{#if onSelectTool}
					<ChevronRight size={13} class="shrink-0 text-tertiary" />
				{/if}
			</button>
		{/each}
	</div>
	{#if onAddTool}
		<div class="mt-2">{@render addToolButton()}</div>
	{/if}
{/if}
