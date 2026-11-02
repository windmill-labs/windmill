<!--
@component
A sidebar section's title row. Only its chevron folds the section, so the title is plain text; its
padding lines the text up with the nav rows' icons and its height matches theirs, so actions are
`sm` (h-7) buttons. The icon rail keeps the row's slot, as a short rule or the section's rail
action, so the rows under it sit where they sit in the open sidebar.
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
		/** In the icon rail: the slot shows `railAction`, or a rule. */
		isCollapsed?: boolean
		railAction?: Snippet
	}

	let {
		title,
		collapsed,
		onToggle,
		disabled = false,
		actions,
		isCollapsed = false,
		railAction
	}: Props = $props()
</script>

{#if isCollapsed}
	<div class="flex flex-row items-center h-7" aria-hidden={railAction ? undefined : 'true'}>
		{#if railAction}
			{@render railAction()}
		{:else}
			<span class="ml-2 w-4 h-px bg-border-light"></span>
		{/if}
	</div>
{:else}
	<div class="flex flex-row items-center h-7 gap-0.5">
		<span class="px-2 text-secondary text-[0.5rem] font-normal uppercase truncate">{title}</span>
		{#if !disabled}
			<Button
				variant="subtle"
				unifiedSize="2xs"
				iconOnly
				startIcon={{ icon: collapsed ? ChevronRight : ChevronDown }}
				title={collapsed ? `Unfold ${title}` : `Fold ${title}`}
				aria-label={collapsed ? `Unfold ${title}` : `Fold ${title}`}
				aria-expanded={!collapsed}
				btnClasses="-ml-1.5 text-secondary"
				onClick={onToggle}
			/>
		{/if}
		<span class="flex-1"></span>
		{@render actions?.()}
	</div>
{/if}
