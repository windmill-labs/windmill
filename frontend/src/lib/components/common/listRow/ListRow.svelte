<!--
@component
One row of a selectable list: an optional leading icon, a label line, one line of
secondary text under it, and optionally controls at the end that act on their own.

Borderless — rows sit in a `flex flex-col gap-1` and light up on hover rather than
living in a bordered card, which reads as heavy once a list runs to dozens of rows.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'

	let {
		title,
		subtitle,
		icon,
		trailing,
		onClick,
		onMouseEnter,
		highlighted,
		id,
		aiId,
		aiDescription,
		class: clazz = ''
	}: {
		/** The label line. A snippet so a caller can highlight a search match, or set a
		 * second identifier beside the name. */
		title: Snippet
		/** One line under the title, truncated. */
		subtitle?: Snippet
		icon?: Snippet
		/** Controls at the end of the row that act on their own — a switch, a menu. A row
		 * that has them is a div with the label as an inner button, since a button cannot
		 * nest inside a button and a click would otherwise fire both. */
		trailing?: Snippet
		onClick?: () => void
		onMouseEnter?: () => void
		/** Given, the caller owns which row is lit — it moves the highlight with the
		 * keyboard — and the row's own hover is off: two lit rows at once are ambiguous. */
		highlighted?: boolean
		id?: string
		aiId?: string
		aiDescription?: string
		class?: string
	} = $props()

	let ownHover = $derived(highlighted === undefined)

	// Set at hover time rather than up front, so only the rows that actually cut their
	// text off carry a tooltip.
	function titleIfTruncated(e: MouseEvent & { currentTarget: HTMLElement }) {
		const el = e.currentTarget
		el.title = el.scrollWidth > el.clientWidth ? (el.textContent?.trim() ?? '') : ''
	}
</script>

{#snippet body()}
	<div class="flex flex-row items-center gap-4 w-full min-w-0 text-left">
		{#if icon}
			<div class="shrink-0">{@render icon()}</div>
		{/if}
		<div class="flex flex-col gap-0.5 min-w-0 grow">
			<div class="flex flex-row items-baseline gap-2 min-w-0">
				{@render title()}
			</div>
			{#if subtitle}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<span
					class="truncate text-xs font-normal leading-4 text-secondary"
					onmouseenter={titleIfTruncated}
				>
					{@render subtitle()}
				</span>
			{/if}
		</div>
	</div>
{/snippet}

{#if trailing}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		{id}
		class={twMerge(
			'flex items-center gap-4 rounded-md px-3 py-3 scroll-my-2 transition-colors text-xs font-medium',
			// Only a row that opens something lights up: a hover on a row whose label does
			// nothing reads as an affordance that isn't there.
			onClick ? (ownHover ? 'hover:bg-surface-hover' : highlighted ? 'bg-surface-hover' : '') : '',
			clazz
		)}
		onmouseenter={onMouseEnter}
	>
		{#if onClick}
			<!-- Its own padding and hover are off: both belong to the row around it, which
			     is what lights up and what the trailing controls sit inside. -->
			<Button
				type="button"
				{aiId}
				{aiDescription}
				variant="subtle"
				unifiedSize="md"
				wrapperClasses="grow shrink min-w-0"
				btnClasses="w-full min-w-0 justify-start p-0 h-auto bg-transparent hover:bg-transparent"
				{onClick}
			>
				{@render body()}
			</Button>
		{:else}
			<!-- No `onClick`: a button here would be a tab stop that does nothing. -->
			<div class="flex grow min-w-0 text-left">{@render body()}</div>
		{/if}
		{@render trailing()}
	</div>
{:else if !onClick}
	<!-- Nothing to click and nothing trailing: a row that only reports. A `Button` here
	     would be a tab stop that lights up on hover and does nothing. -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div {id} class={twMerge('px-3 py-3 text-xs font-medium', clazz)} onmouseenter={onMouseEnter}>
		{@render body()}
	</div>
{:else}
	<Button
		type="button"
		{id}
		{aiId}
		{aiDescription}
		variant="subtle"
		unifiedSize="md"
		btnClasses={twMerge(
			'justify-start px-3 h-auto py-3 scroll-my-2',
			ownHover ? '' : 'hover:bg-transparent',
			// `!` so the highlight also wins on the row the pointer is over, whose own
			// hover was turned off just above.
			highlighted ? '!bg-surface-hover' : '',
			clazz
		)}
		on:mouseenter={() => onMouseEnter?.()}
		{onClick}
	>
		{@render body()}
	</Button>
{/if}
