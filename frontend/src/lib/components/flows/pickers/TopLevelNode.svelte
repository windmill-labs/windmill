<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import { CheckCircle2, ChevronRight, Code, GitBranch, Repeat, Square, Zap } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let label: string
	export let selected = false
	export let returnIcon = false
	const dispatch = createEventDispatcher()
</script>

<button
	class={twMerge(
		'w-full text-left py-2 px-1.5 hover:bg-surface-hover text-xs font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
		selected ? 'bg-surface-hover' : '',
		$$props.class
	)}
	on:pointerdown={() => dispatch('select', label)}
	role="menuitem"
	tabindex="-1"
>
	<span class="grow flex items-center gap-2">
		{#if label === 'Action'}
			<Code size={14} />
			Action
			<ChevronRight size={12} class="ml-auto" color="#4c566a" />
		{:else if label === 'Trigger'}
			<Zap size={14} />
			Trigger
			<ChevronRight size={12} class="ml-auto" color="#4c566a" />
		{:else if label === 'Approval/Prompt'}
			<CheckCircle2 size={14} />
			Approval/Prompt
			<ChevronRight size={12} class="ml-auto" color="#4c566a" />
		{:else if label === 'Flow'}
			<BarsStaggered size={14} />
			Flow
			<ChevronRight size={12} class="ml-auto" color="#4c566a" />
		{:else if label === 'End Flow'}
			<Square size={14} />
			End Flow
		{:else if label === 'For Loop'}
			<Repeat size={14} />
			For Loop
		{:else if label === 'While Loop'}
			<Repeat size={14} />
			While Loop
		{:else if label === 'Branch to one'}
			<GitBranch size={14} />
			Branch to one
		{:else if label === 'Branch to all'}
			<GitBranch size={14} />
			Branch to all
		{/if}
	</span>
	{#if returnIcon && selected}
		<kbd class="!text-xs text-right">&crarr;</kbd>
	{/if}
</button>
