<script lang="ts">
	import { Circle, CircleDot } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

	let {
		label,
		description = undefined,
		selected = false,
		onSelect,
		disabled = false,
		icon = undefined,
		class: className = ''
	}: {
		/** Title shown in bold at the top of the card */
		label: string
		/** Optional supporting line under the label */
		description?: string
		/** Whether this card is the selected option */
		selected?: boolean
		/** Called when the card is clicked */
		onSelect: () => void
		disabled?: boolean
		/** Optional leading icon, rendered after the radio */
		icon?: Snippet
		class?: string
	} = $props()
</script>

<button
	type="button"
	{disabled}
	onclick={onSelect}
	class={twMerge(
		'w-full text-left rounded-md border p-3 transition-colors',
		selected
			? 'border-border-selected bg-surface-selected'
			: 'border-border-light hover:bg-surface-hover',
		disabled && 'opacity-50 cursor-not-allowed',
		className
	)}
>
	<div class="flex items-start gap-2">
		{#if selected}
			<CircleDot size={16} class="text-accent shrink-0 mt-0.5" />
		{:else}
			<Circle size={16} class="text-hint shrink-0 mt-0.5" />
		{/if}
		{#if icon}
			<div class="shrink-0 mt-0.5">{@render icon()}</div>
		{/if}
		<div class="flex-1 min-w-0">
			<div class="text-xs font-semibold text-emphasis">{label}</div>
			{#if description}
				<div class="text-xs font-normal text-secondary mt-0.5">{description}</div>
			{/if}
		</div>
	</div>
</button>
