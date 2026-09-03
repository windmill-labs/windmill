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
		showRadio = true,
		class: className = ''
	}: {
		/** Title shown in bold at the top of the card */
		label: string
		/** Optional supporting line under the label. A snippet when it needs markup
		 * of its own — an emphasised name, a count — rather than plain text. */
		description?: string | Snippet
		/** Whether this card is the selected option */
		selected?: boolean
		/** Called when the card is clicked */
		onSelect: () => void
		disabled?: boolean
		/** Optional leading icon, rendered after the radio */
		icon?: Snippet
		/** Draw the radio glyph. Turn it off where the card itself is the only
		 * control and the border and tint already say which one is picked — the dot
		 * is then a second, redundant answer to the same question. The group still
		 * reads as radios to a screen reader, which is what `role` carries. */
		showRadio?: boolean
		class?: string
	} = $props()

	// A snippet is a function; a description string is not. Checked rather than
	// requiring callers to pick between two props.
	const describedBySnippet = $derived(typeof description === 'function')
</script>

<button
	type="button"
	{disabled}
	role="radio"
	aria-checked={selected}
	onclick={onSelect}
	class={twMerge(
		'w-full text-left rounded-md border p-3 transition-colors',
		// `surface-accent-selected`, not the neutral `surface-selected`: this is the
		// token the rest of the app uses to say "this is the one you picked"
		// (FileExplorer, TriggersTable, RunnableRow, ArtifactVersionPicker).
		selected
			? 'border-border-selected bg-surface-accent-selected'
			: 'border-border-light hover:bg-surface-hover',
		disabled && 'opacity-50 cursor-not-allowed',
		className
	)}
>
	<div class="flex items-start gap-2">
		{#if showRadio}
			{#if selected}
				<CircleDot size={16} class="text-accent shrink-0 mt-0.5" />
			{:else}
				<Circle size={16} class="text-hint shrink-0 mt-0.5" />
			{/if}
		{/if}
		{#if icon}
			<div class="shrink-0 mt-0.5">{@render icon()}</div>
		{/if}
		<div class="flex-1 min-w-0">
			<div class="text-xs font-semibold text-emphasis">{label}</div>
			{#if describedBySnippet}
				<div class="text-xs font-normal text-secondary mt-0.5">
					{@render (description as Snippet)()}
				</div>
			{:else if description}
				<div class="text-xs font-normal text-secondary mt-0.5">{description}</div>
			{/if}
		</div>
	</div>
</button>
