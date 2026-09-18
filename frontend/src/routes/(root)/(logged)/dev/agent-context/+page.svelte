<script lang="ts">
	// Dev-only design page for the context summary an empty session chat shows. Not
	// linked anywhere; open at /dev/agent-context. Mock data only, so the shapes a real
	// workspace rarely has (nothing configured, a dozen skills) can be looked at.
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ContextSummaryRows from '$lib/components/copilot/chat/contextSummary/ContextSummaryRows.svelte'
	import type {
		AgentContext,
		ContextSummaryActions
	} from '$lib/components/copilot/chat/contextSummary/contextSummary'
	import { sendUserToast } from '$lib/toast'
	import { ArrowUp, Paperclip } from 'lucide-svelte'
	import { MOCK_CONTEXTS } from './mockContexts'

	let scenario = $state(MOCK_CONTEXTS[1].id)
	let ctx = $state<AgentContext>(structuredClone(MOCK_CONTEXTS[1].ctx))

	function selectScenario(id: string) {
		const found = MOCK_CONTEXTS.find((m) => m.id === id)
		if (found) ctx = structuredClone(found.ctx)
	}

	const actions: ContextSummaryActions = {
		onToggleSkill: (path, enabled) => {
			const skill = ctx.skills.find((s) => s.path === path)
			if (skill) skill.enabled = enabled
		},
		onToggleMcp: (path, enabled) => {
			const server = ctx.mcpServers.find((s) => s.path === path)
			if (server) server.enabled = enabled
		},
		onManage: (section) => sendUserToast(`Would open assistant settings on "${section}"`)
	}
</script>

{#if import.meta.env.DEV}
	<div class="h-full overflow-auto p-6 bg-surface">
		<div class="max-w-[720px] mx-auto flex flex-col gap-4">
			<div class="flex flex-col gap-1">
				<h1 class="text-lg font-semibold text-emphasis">Agent context summary</h1>
				<p class="text-xs text-secondary">
					Dev-only mock of what an empty session chat shows about its agent's context.
				</p>
			</div>
			<ToggleButtonGroup bind:selected={scenario} onSelected={selectScenario} noWFull>
				{#snippet children({ item })}
					{#each MOCK_CONTEXTS as m (m.id)}
						<ToggleButton value={m.id} label={m.label} {item} small />
					{/each}
				{/snippet}
			</ToggleButtonGroup>

			<div
				class="h-[560px] rounded-md border border-light bg-surface-primary flex flex-col overflow-hidden"
			>
				<div class="px-4 py-2 text-sm font-semibold text-hint shrink-0">Untitled session</div>
				<!-- Auto margins rather than justify-*: justify clips content taller than the
				     scroller. -->
				<div class="flex-1 min-h-0 overflow-y-auto px-4 flex flex-col">
					<div class="w-full max-w-xl mx-auto py-2 my-auto">
						<ContextSummaryRows {ctx} {actions} />
					</div>
				</div>
				<div class="px-4 pb-3 pt-1 shrink-0">
					<div
						class="max-w-xl mx-auto rounded-md border border-light bg-surface-input px-3 py-2 flex items-center gap-2"
					>
						<span class="text-xs text-hint grow">Ask anything…</span>
						<Paperclip size={14} class="text-hint" />
						<ArrowUp size={14} class="text-hint" />
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
