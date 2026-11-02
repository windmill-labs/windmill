<!--
@component
A foldable sidebar section whose rows keep their places in every sidebar mode. Unfolded: its title
row over its items. Folded: one row with the section's icon, whose items open in a popover on
hover, or pinned there by a click. A fold is a layout the reader keeps, so reaching a folded
section's items never unfolds it: only the chevron, or the popover's Unfold, does.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { slide } from 'svelte/transition'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { debounce } from '$lib/utils'
	import SidebarSectionHeader from './SidebarSectionHeader.svelte'
	import { sidebarClasses } from './MenuButton.svelte'

	interface Props {
		title: string
		icon: any
		folded: boolean
		onToggle: () => void
		isCollapsed: boolean
		/** How many items the folded row holds, shown beside its title. */
		count?: number
		/** A dot on the folded row's icon: something inside wants attention. */
		notify?: boolean
		/** A section with nothing to fold: the title stays, without its chevron. */
		disabled?: boolean
		actions?: Snippet
		railAction?: Snippet
		/** The section's items; `inPopover` asks for them with their names, whatever the mode. */
		children: Snippet<[boolean]>
	}

	let {
		title,
		icon: Icon,
		folded,
		onToggle,
		isCollapsed,
		count = undefined,
		notify = false,
		disabled = false,
		actions,
		railAction,
		children
	}: Props = $props()

	// The open delay keeps a pointer moving down the sidebar from opening every folded section it
	// crosses; the close delay lets it cross the gap into the popover.
	const OPEN_DELAY_MS = 120
	const CLOSE_DELAY_MS = 150
	let open = $state(false)
	let pinned = $state(false)
	const { debounced: scheduleOpen, clearDebounce: cancelOpen } = debounce(
		() => (open = true),
		OPEN_DELAY_MS
	)
	const { debounced: scheduleClose, clearDebounce: cancelClose } = debounce(() => {
		if (!pinned) open = false
	}, CLOSE_DELAY_MS)
	function onEnter() {
		cancelClose()
		if (!open) scheduleOpen()
	}
	function onLeave() {
		cancelOpen()
		scheduleClose()
	}
	function closeNow() {
		cancelOpen()
		pinned = false
		open = false
	}
</script>

{#if folded && !disabled}
	<div class="relative">
		<!-- A click pins the popover. Caught before melt's own toggle, which would close one the
		     hover just opened. -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onmouseenter={onEnter}
			onmouseleave={onLeave}
			onclickcapture={(e) => {
				cancelOpen()
				if (open && !pinned) {
					e.stopPropagation()
					pinned = true
				} else if (!open) {
					pinned = true
				}
			}}
		>
			<Popover
				bind:isOpen={open}
				placement="right-start"
				disableFocusTrap
				openFocus={null}
				closeFocus={null}
				onClose={() => (pinned = false)}
				extraProps={{ 'data-sidebar-flyout': '' }}
				triggerAttrs={{ 'aria-label': count ? `${title} (${count})` : title, title }}
				class={twMerge(
					'relative flex items-center w-full h-8 px-2 gap-2 rounded-md text-left',
					sidebarClasses.text,
					open ? sidebarClasses.selectedBg : sidebarClasses.hoverBg
				)}
			>
				{#snippet trigger()}
					<Icon size={16} class={twMerge('flex-shrink-0', sidebarClasses.iconText)} />
					{#if notify}
						<span
							class="absolute top-1.5 left-[22px] w-1.5 h-1.5 rounded-full bg-surface-accent-primary"
						></span>
					{/if}
					{#if isCollapsed}
						<!-- The rail has no title: the dot under the icon says the row holds more. -->
						<span
							class="absolute bottom-1 left-[22px] w-1 h-1 rounded-full bg-current text-hint opacity-60"
						></span>
					{:else}
						<span class="truncate">{title}</span>
						{#if count}
							<span class="ml-auto pr-6 text-2xs text-hint tabular-nums">{count}</span>
						{/if}
					{/if}
				{/snippet}
				{#snippet content()}
					<!-- A link or a session picked here leaves the popover with the page it opened. -->
					<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
					<div
						class="flex flex-col gap-0.5 p-1 w-56 max-h-[60vh] overflow-y-auto"
						onmouseenter={onEnter}
						onmouseleave={onLeave}
						onclick={(e) => {
							if ((e.target as Element).closest('a, [role="option"]')) closeNow()
						}}
					>
						<div class="flex flex-row items-center h-7 gap-1 pl-2">
							<span class="flex-1 truncate text-secondary text-[0.5rem] font-normal uppercase"
								>{title}</span
							>
							<Button
								variant="subtle"
								unifiedSize="2xs"
								startIcon={{ icon: ChevronDown }}
								btnClasses="text-secondary"
								onClick={() => {
									closeNow()
									onToggle()
								}}
							>
								Unfold
							</Button>
						</div>
						{@render children(true)}
					</div>
				{/snippet}
			</Popover>
		</div>
		{#if !isCollapsed}
			<div class="absolute right-1 top-1/2 -translate-y-1/2">
				<Button
					variant="subtle"
					unifiedSize="2xs"
					iconOnly
					startIcon={{ icon: ChevronRight }}
					title="Unfold {title}"
					aria-label="Unfold {title}"
					aria-expanded={false}
					btnClasses="text-secondary"
					onClick={() => {
						closeNow()
						onToggle()
					}}
				/>
			</div>
		{/if}
	</div>
{:else}
	<SidebarSectionHeader
		{title}
		collapsed={false}
		{onToggle}
		{disabled}
		{actions}
		{isCollapsed}
		{railAction}
	/>
	{#if !folded || disabled}
		<div transition:slide={{ duration: 180 }}>
			{@render children(false)}
		</div>
	{/if}
{/if}
