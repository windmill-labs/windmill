<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'

	interface Props {
		id: string
		isCollapsible: boolean
		isRunning?: boolean
		isCurrent: (id: string) => boolean
		isExpanded: (id: string, isRunning?: boolean) => boolean
		toggleExpanded: (id: string) => void
		children: import('svelte').Snippet
		label: import('svelte').Snippet
		class?: string
		select: (id: string) => void
	}

	let {
		id,
		isCollapsible,
		isRunning,
		isCurrent,
		isExpanded,
		toggleExpanded,
		children,
		label,
		class: className = '',
		select
	}: Props = $props()

	function handleClick() {
		if (isCollapsible) {
			toggleExpanded(id)
		}
		select(id)
	}
</script>

<li
	class={twMerge(
		'border-b flex flex-col',
		isCurrent(id) ? 'border-l-2 !border-l-gray-400 -ml-[2px]' : ''
	)}
>
	<button
		class={twMerge(
			'py-1 leading-tight w-full flex items-center justify-left text-xs text-primary hover:text-primary hover:bg-surface-hover ',
			isCurrent(id) ? 'bg-surface-hover text-primary' : '',
			className
		)}
		tabindex="-1"
		onclick={handleClick}
		data-nav-id={id}
	>
		<div class="px-1">
			{#if isCollapsible}
				{#if isExpanded(id, isRunning) || id === 'root-flow'}
					<ChevronDown size={8} strokeWidth={isCurrent(id) ? 4 : 2} />
				{:else}
					<ChevronRight size={8} strokeWidth={isCurrent(id) ? 4 : 2} />
				{/if}
			{:else}
				<!-- Empty subflow - no collapse button, just spacing -->
				<div class="w-2"></div>
			{/if}
		</div>
		<div class="w-full leading-tight">
			<div class={twMerge('w-full pr-2 cursor-pointer', isCurrent(id) ? 'bg-surface-hover' : '')}>
				{@render label()}
			</div>
		</div>
	</button>

	{#if isExpanded(id, isRunning) || !isCollapsible}
		<div class="my-1 transition-all duration-200 ease-in-out">
			<div class="pl-4">
				{@render children()}
			</div>
		</div>
	{/if}
</li>

<style>
	.transition-all {
		transition: all 0.2s ease-in-out;
	}

	[data-nav-id]:focus-visible {
		outline: none;
		outline-offset: 0;
	}
</style>
