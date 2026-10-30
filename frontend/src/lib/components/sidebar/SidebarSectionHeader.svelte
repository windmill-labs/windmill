<!--
@component
A sidebar section's title row. The title folds the section and fills the row up to its actions;
its padding lines the text up with the nav rows' icons and its height matches theirs, so actions
are `sm` (h-7) buttons.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		title: string
		collapsed: boolean
		onToggle: () => void
		/** A section with nothing to fold: the title stays, without its chevron. */
		disabled?: boolean
		actions?: Snippet
	}

	let { title, collapsed, onToggle, disabled = false, actions }: Props = $props()

	const titleClasses = 'px-2 text-secondary text-[0.5rem] font-normal uppercase'
</script>

<div class="flex flex-row items-center h-7 gap-0.5">
	{#if disabled}
		<!-- A label rather than a disabled Button, which would grey the title out. -->
		<span class="flex-1 min-w-0 h-7 flex flex-row items-center {titleClasses}">{title}</span>
	{:else}
		<Button
			variant="subtle"
			unifiedSize="sm"
			onClick={onToggle}
			aria-expanded={!collapsed}
			wrapperClasses="flex-1 min-w-0"
			btnClasses="justify-start text-left gap-1 {titleClasses}"
		>
			{title}
			{#if collapsed}
				<ChevronRight size={10} />
			{:else}
				<ChevronDown size={10} />
			{/if}
		</Button>
	{/if}
	{@render actions?.()}
</div>
