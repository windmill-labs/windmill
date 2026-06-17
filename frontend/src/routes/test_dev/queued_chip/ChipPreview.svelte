<script lang="ts">
	import { setContext } from 'svelte'
	import AIChatInput from '$lib/components/copilot/chat/AIChatInput.svelte'
	import QueuedMessageChip from '$lib/components/copilot/chat/QueuedMessageChip.svelte'
	import { AIChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import type { ContextElement } from '$lib/components/copilot/chat/context'
	import { Button } from '$lib/components/common'
	import { ArrowRight, DiffIcon, GitFork, Pencil } from 'lucide-svelte'

	let {
		queued,
		mode = AIMode.GLOBAL,
		draft = '',
		label,
		bars = [],
		context = []
	}: {
		queued: string
		mode?: AIMode
		draft?: string
		label: string
		// Static replicas of the session bars (SessionForkBar / SessionDraftBar)
		// that SessionWrapper renders above the input via `inputPreface` — the
		// real ones need a live session runtime, so the dev page mocks the markup.
		bars?: ('fork' | 'draft')[]
		context?: ContextElement[]
	} = $props()

	// Dedicated manager frozen in the "turn is streaming" state so the
	// queued-message chip renders. loading stays true, so typing + Enter in
	// the input below queues live — handy for testing append behavior.
	const manager = new AIChatManager()
	manager.mode = mode
	manager.loading = true
	manager.queuedMessage = queued
	setContext<AIChatManager>('aiChatManager', manager)

	let selectedContext = $state(context)
	let inputComponent: AIChatInput | undefined = $state()
	// Wire the input ref like AIChatDisplay does, so the chip's X-restore
	// (manager.aiChatInput.prependText) works on the dev page too.
	$effect(() => {
		if (inputComponent) {
			// `as any`: svelte-fast-check types cross-file `bind:this` refs as an
			// opaque `Comp`, which doesn't unify with the instance type the
			// manager expects. Dev-only page, runtime shape is correct.
			manager.setAiChatInput(inputComponent as any)
		}
	})
</script>

<div class="flex flex-col gap-1">
	<span class="text-xs text-tertiary">{label}</span>
	<div class="w-[400px] border rounded-lg p-2 bg-surface flex flex-col gap-1">
		<QueuedMessageChip />
		{#if bars.includes('fork')}
			<div
				class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
			>
				<div class="flex items-center gap-1.5 min-w-0">
					<span class="inline-flex shrink-0"><GitFork class="w-3.5 h-3.5 text-secondary" /></span>
					<span class="truncate text-secondary">my-feature-fork</span>
					<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
					<span class="truncate text-secondary">parent-workspace</span>
				</div>
				<div class="flex items-center gap-1 shrink-0">
					<Button variant="default" unifiedSize="xs" startIcon={{ icon: DiffIcon }}>3</Button>
					<Button variant="default" unifiedSize="xs">Review</Button>
				</div>
			</div>
		{/if}
		{#if bars.includes('draft')}
			<div
				class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
			>
				<div class="flex items-center gap-1.5 min-w-0">
					<span class="inline-flex shrink-0"><Pencil class="w-3.5 h-3.5 text-secondary" /></span>
					<span class="truncate text-secondary">2 drafts</span>
				</div>
				<div class="flex items-center gap-1 shrink-0">
					<Button variant="default" unifiedSize="xs" startIcon={{ icon: DiffIcon }}>2</Button>
					<Button variant="default" unifiedSize="xs">Review</Button>
				</div>
			</div>
		{/if}
		<AIChatInput
			bind:this={inputComponent}
			availableContext={selectedContext}
			bind:selectedContext
			initialInstructions={draft}
		/>
	</div>
</div>
