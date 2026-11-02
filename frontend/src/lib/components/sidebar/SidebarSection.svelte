<!--
@component
A foldable sidebar section whose rows keep their places in every sidebar mode. Its title row is the
same row folded or not, so folding moves nothing but the items under it. The row behaves the same in
every mode: a click folds or unfolds it, and hovering a folded row shows its items in a popover.
A fold is a layout the reader keeps, so reaching a folded section's items never unfolds it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { slide } from 'svelte/transition'
	import { ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { debounce } from '$lib/utils'
	import { sidebarClasses } from './MenuButton.svelte'

	interface Props {
		title: string
		icon: any
		folded: boolean
		onToggle: () => void
		isCollapsed: boolean
		/** A dot on the icon: something inside wants attention. */
		notify?: boolean
		/** A section with nothing to fold: the row stays, without its chevron. */
		disabled?: boolean
		/**
		 * Buttons at the end of an unfolded row in the open sidebar, and at the top of a folded row's
		 * popover. A menu they open from the popover sits outside it, so `hold(true)` keeps the popover
		 * open until `hold(false)`.
		 */
		actions?: Snippet<[{ inPopover: boolean; hold: (held: boolean) => void }]>
		/** The section's items; `inPopover` asks for them with their names, whatever the mode. */
		children: Snippet<[boolean]>
	}

	let {
		title,
		icon: Icon,
		folded,
		onToggle,
		isCollapsed,
		notify = false,
		disabled = false,
		actions,
		children
	}: Props = $props()

	const isFolded = $derived(folded && !disabled)
	const toggleLabel = $derived(isFolded ? `Unfold ${title}` : `Fold ${title}`)
	const rowClass =
		'relative flex items-center justify-start w-full h-[30px] px-2 gap-2 rounded-lg text-left font-normal'

	// The open delay keeps a pointer moving down the sidebar from opening every folded section it
	// crosses; the close delay lets it cross the gap into the popover.
	const OPEN_DELAY_MS = 120
	const CLOSE_DELAY_MS = 150
	let open = $state(false)
	let held = $state(false)
	let popoverContent: HTMLElement | undefined = $state()
	const { debounced: scheduleOpen, clearDebounce: cancelOpen } = debounce(
		() => (open = true),
		OPEN_DELAY_MS
	)
	const { debounced: scheduleClose, clearDebounce: cancelClose } = debounce(() => {
		if (!held) open = false
	}, CLOSE_DELAY_MS)
	function onEnter() {
		cancelOpen()
		cancelClose()
		if (!open) scheduleOpen()
	}
	function onLeave() {
		cancelOpen()
		scheduleClose()
	}
	function closeNow() {
		cancelOpen()
		cancelClose()
		held = false
		open = false
	}
	function hold(next: boolean) {
		held = next
		if (!next && !popoverContent?.matches(':hover')) scheduleClose()
	}
	function toggle() {
		closeNow()
		onToggle()
	}
</script>

<!-- The fold chevron takes an item icon's place, so the row reads as something to open. The title
     names the section in the open sidebar, and an unfolded section's items name it in the rail; a
     folded section in the rail has neither, so the section's icon marks the chevron's corner. With
     nothing to fold, the rail shows the icon alone. -->
{#snippet iconContent()}
	<span class="relative flex-shrink-0 flex items-center justify-center w-4 h-4">
		{#if disabled}
			{#if isCollapsed}
				<Icon size={16} class="text-hint" />
			{/if}
		{:else}
			<!-- 2.67 keeps the 1.33px stroke a 16px item icon draws at strokeWidth 2. -->
			<ChevronRight
				size={12}
				strokeWidth={2.67}
				class={twMerge('text-hint transition-transform duration-200', !isFolded && 'rotate-90')}
				aria-hidden="true"
			/>
			{#if isCollapsed && isFolded}
				<Icon size={9} strokeWidth={2.5} class="absolute -right-1 -bottom-1 text-hint" />
			{/if}
		{/if}
		{#if notify}
			<span class="absolute -top-1 -right-1 w-1.5 h-1.5 rounded-full bg-surface-accent-primary"
			></span>
		{/if}
	</span>
{/snippet}

{#snippet titleContent()}
	{#if !isCollapsed}
		<!-- The chevron draws narrower than an item icon, so the items' 8px gap would read as more. -->
		<span class="-ml-1 truncate uppercase text-[0.5rem] font-normal text-secondary">{title}</span>
	{/if}
{/snippet}

{#snippet rowContent()}
	{@render iconContent()}
	{@render titleContent()}
{/snippet}

<!-- Padding, not a margin, widens the gap above the section: a margin would collapse into the
     wrapper's own. The card has no side padding, so its rows keep the places they have outside it;
     it darkens whatever the sidebar's color is, accent tint included. -->
<div class="pt-1.5">
	<div
		class={twMerge(
			'rounded-lg bg-black/[0.02] dark:bg-white/[0.025]',
			// Folded, the card is the row alone, so the row's hover fills it exactly.
			!isFolded && 'pb-0.5'
		)}
	>
		<!-- A fixed height and flex wrappers: Firefox gives inline wrappers line space, so a row sized by
	     its content comes out taller than the 30px every other row has, more so while it is squeezed. -->
		<div class="relative flex flex-row items-center gap-0.5 h-[30px]">
			<div class="flex-1 min-w-0 flex items-center">
				{#if isFolded}
					<!-- The click folds rather than toggling the popover: caught before melt's own handler. -->
					<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
					<div
						class={twMerge(
							rowClass,
							'px-0',
							open ? sidebarClasses.selectedBg : sidebarClasses.hoverBg
						)}
						onmouseenter={onEnter}
						onmouseleave={onLeave}
						onclickcapture={(e) => {
							e.stopPropagation()
							toggle()
						}}
					>
						<Popover
							bind:isOpen={open}
							placement="right-start"
							disableFocusTrap
							openFocus={null}
							closeFocus={null}
							closeOnOutsideClick={!held}
							extraProps={{ 'data-sidebar-flyout': '' }}
							triggerAttrs={{
								'aria-label': toggleLabel,
								'aria-expanded': false,
								title: toggleLabel
							}}
							class="w-full h-full min-w-0 flex items-center gap-2 px-2 text-left"
						>
							{#snippet trigger()}
								{@render rowContent()}
							{/snippet}
							{#snippet content()}
								<!-- A link or a session picked here leaves the popover with the page it opened. -->
								<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
								<div
									bind:this={popoverContent}
									class="flex flex-col gap-0.5 p-1 w-56 max-h-[60vh] overflow-y-auto"
									onmouseenter={onEnter}
									onmouseleave={onLeave}
									onclick={(e) => {
										if ((e.target as Element).closest('a, [role="option"]')) closeNow()
									}}
								>
									<!-- Icons only shows no title on the row, so its popover does. A folded row
									     shows no actions either, so they open the popover's list. -->
									{#if isCollapsed}
										<div class="flex flex-row items-center h-7 pl-2">
											<span
												class="flex-1 truncate text-secondary text-[0.5rem] font-normal uppercase"
												>{title}</span
											>
										</div>
									{/if}
									{@render actions?.({ inPopover: true, hold })}
									{@render children(true)}
								</div>
							{/snippet}
						</Popover>
					</div>
				{:else if disabled}
					<div class={rowClass}>
						{@render rowContent()}
					</div>
				{:else}
					<Button
						variant="subtle"
						title={toggleLabel}
						aria-label={toggleLabel}
						aria-expanded={true}
						wrapperClasses="w-full"
						btnClasses={twMerge(rowClass, sidebarClasses.hoverBg)}
						onClick={toggle}
					>
						{@render rowContent()}
					</Button>
				{/if}
			</div>
			{#if !isCollapsed && !isFolded}
				{@render actions?.({ inPopover: false, hold })}
			{/if}
		</div>
		{#if !isFolded}
			<div transition:slide={{ duration: 180 }}>
				{@render children(false)}
			</div>
		{/if}
	</div>
</div>
