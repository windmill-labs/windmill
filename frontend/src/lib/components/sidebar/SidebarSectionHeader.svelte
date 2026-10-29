<!--
@component
A sidebar section's title row. The title folds the section and fills the row up to its actions;
its padding lines the text up with the nav rows' icons and its height matches theirs, so actions
are `sm` (h-7) buttons.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'

	interface Props {
		title: string
		collapsed: boolean
		onToggle: () => void
		/** A section with nothing to fold: the title stays, without its chevron. */
		disabled?: boolean
		actions?: Snippet
	}

	let { title, collapsed, onToggle, disabled = false, actions }: Props = $props()
</script>

<div class="flex flex-row items-center h-7 gap-0.5">
	<button
		type="button"
		onclick={onToggle}
		{disabled}
		aria-expanded={!collapsed}
		class="flex-1 min-w-0 h-7 px-2 rounded-md text-secondary text-[0.5rem] font-normal uppercase flex flex-row items-center gap-1 transition-colors hover:bg-surface-hover disabled:hover:bg-transparent focus:outline-none"
	>
		{title}
		{#if !disabled}
			{#if collapsed}
				<ChevronRight size={10} />
			{:else}
				<ChevronDown size={10} />
			{/if}
		{/if}
	</button>
	{@render actions?.()}
</div>
