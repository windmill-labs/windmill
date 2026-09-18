<script lang="ts">
	import { ChevronRight } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import ContextSectionDetail from './ContextSectionDetail.svelte'
	import {
		summarizeContext,
		type AgentContext,
		type ContextSectionId,
		type ContextSummaryActions
	} from './contextSummary'
	import { CONTEXT_SECTION_ICONS } from './sectionIcons'

	let {
		ctx,
		actions,
		toggleBlockedReason
	}: {
		ctx: AgentContext
		actions: ContextSummaryActions
		toggleBlockedReason?: string
	} = $props()

	let sections = $derived(summarizeContext(ctx))
	// Each row opens and closes on its own: comparing two sections means having both open.
	let open = $state<Partial<Record<ContextSectionId, boolean>>>({})
</script>

<div class="flex flex-col w-full">
	<span class="text-2xs text-secondary px-2 pb-1">Context</span>
	{#each sections as s (s.id)}
		{@const Icon = CONTEXT_SECTION_ICONS[s.id]}
		<button
			type="button"
			class="flex items-center gap-2 px-2 py-1 rounded-md hover:bg-surface-hover text-left min-w-0"
			aria-expanded={open[s.id] ?? false}
			onclick={() => (open[s.id] = !open[s.id])}
		>
			<ChevronRight
				size={12}
				class="shrink-0 text-hint transition-transform {open[s.id] ? 'rotate-90' : ''}"
			/>
			<Icon size={12} class="shrink-0 {s.empty ? 'text-hint' : 'text-tertiary'}" />
			<span class="text-xs w-20 shrink-0 {s.empty ? 'text-hint' : 'text-emphasis'}">{s.label}</span>
			<span class="text-xs w-12 shrink-0 tabular-nums {s.empty ? 'text-hint' : 'text-secondary'}"
				>{s.count ?? ''}</span
			>
			<span class="text-2xs text-hint truncate min-w-0">
				{s.names.length ? s.names.join(', ') : s.emptyText}
			</span>
		</button>
		{#if open[s.id]}
			<div class="pl-10 pr-2 pt-1 pb-3" transition:slide|local={{ duration: 150 }}>
				<ContextSectionDetail {ctx} section={s.id} {actions} {toggleBlockedReason} />
			</div>
		{/if}
	{/each}
</div>
