<!--
@component
A foldable sidebar section whose rows keep their places in every sidebar mode. Its title row is the
same row folded or not, so folding moves nothing but the items under it. The row behaves the same in
every mode: a click folds or unfolds it, and hovering a folded row shows its items in a menu.
A fold is a layout the reader keeps, so reaching a folded section's items never unfolds it.
-->
<script module lang="ts">
	import type { MenubarMenuElements, createDropdownMenu } from '@melt-ui/svelte'

	/** The folded section's menu, for rows that should be its items. */
	export type SectionMenu = {
		item: MenubarMenuElements['item']
		builders: ReturnType<typeof createDropdownMenu>['builders']
	}
	export type SectionActionsArgs = { inPopover: boolean; menu?: SectionMenu }
	/** A row of the folded section's menu that is not a link, styled as its links are. */
	export const flyoutRowClass =
		'h-[30px] px-2 py-0 rounded-md text-xs font-normal text-secondary [&>svg:first-child]:text-hint'
</script>

<script lang="ts">
	import type { Snippet } from 'svelte'
	import { slide } from 'svelte/transition'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button } from '$lib/components/common'
	import Menubar from '$lib/components/meltComponents/Menubar.svelte'
	import Menu from '$lib/components/meltComponents/Menu.svelte'
	import MeltButton from '$lib/components/meltComponents/MeltButton.svelte'
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
		 * Buttons at the end of an unfolded row in the open sidebar, and rows at the top of a folded
		 * row's menu, where `menu` lets them be its items and open submenus from it.
		 */
		actions?: Snippet<[SectionActionsArgs]>
		/**
		 * The section's items; `inPopover` asks for them with their names, whatever the mode, and
		 * `menu` is the folded row's menu they are listed in.
		 */
		children: Snippet<[boolean, SectionMenu | undefined]>
		/** The folded row's menu closed. */
		onClose?: () => void
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
		// Renamed: the menu's own `children` snippets below would shadow it.
		children: items,
		onClose
	}: Props = $props()

	const isFolded = $derived(folded && !disabled)
	const toggleLabel = $derived(isFolded ? `Unfold ${title}` : `Fold ${title}`)
	const rowClass =
		'relative flex items-center justify-start w-full h-[30px] px-2 gap-2 rounded-lg text-left font-normal'

	let open = $state(false)
	function setOpen(value: boolean) {
		if (open && !value) onClose?.()
		open = value
	}
	function toggle() {
		setOpen(false)
		onToggle()
	}
</script>

<!-- In the open sidebar a section row is its title at the start and a chevron at the end, down
     when unfolded, right when folded. The rail has no title, so the row takes an item icon's place:
     the down chevron when unfolded (its items, right under it, name it), the section's icon when
     folded, with a right chevron in the strip beside it. -->
{#snippet foldChevron(size: number, extraClass: string)}
	<ChevronRight {size} class={twMerge('flex-shrink-0 text-hint', extraClass)} aria-hidden="true" />
{/snippet}

{#snippet notifyDot(extraClass: string)}
	<span class={twMerge('w-1.5 h-1.5 rounded-full bg-surface-accent-primary', extraClass)}></span>
{/snippet}

{#snippet sectionIcon()}
	<!-- Under an item icon's 16px, to leave the rail's chevron room beside it. -->
	<span class="relative flex-shrink-0 flex">
		<Icon size={12} class="text-hint" />
		{#if notify}
			{@render notifyDot('absolute -top-1 -right-1')}
		{/if}
	</span>
{/snippet}

{#snippet unfoldedChevron(extraClass: string)}
	<!-- 2.67 keeps the 1.33px stroke a 16px item icon draws at strokeWidth 2. -->
	<ChevronDown
		size={12}
		strokeWidth={2.67}
		class={twMerge('flex-shrink-0 text-hint', extraClass)}
		aria-hidden="true"
	/>
{/snippet}

{#snippet rowContent()}
	{#if isCollapsed}
		<span class="flex-shrink-0 flex items-center justify-center w-4 h-4">
			{#if isFolded || disabled}
				{@render sectionIcon()}
			{:else}
				{@render unfoldedChevron('')}
			{/if}
		</span>
		{#if isFolded}
			{@render foldChevron(10, 'absolute right-0.5 top-[10px]')}
		{/if}
	{:else if isFolded}
		{@render foldChevron(12, 'ml-auto')}
	{:else if !disabled}
		{@render unfoldedChevron('ml-auto')}
	{/if}
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
			<div class="relative flex-1 min-w-0 flex items-center">
				{#if isFolded}
					<!-- The click folds rather than pinning the menu: caught before melt's own handler. Only
					     the user's: the menu opens on hover by clicking its own trigger from script. -->
					<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
					<div
						class={twMerge(
							rowClass,
							'px-0',
							open ? sidebarClasses.selectedBg : sidebarClasses.hoverBg
						)}
						onclickcapture={(e) => {
							if (!e.isTrusted) return
							e.stopPropagation()
							toggle()
						}}
					>
						<Menubar class="w-full h-full">
							{#snippet children({ createMenu })}
								<Menu
									{createMenu}
									bind:open={() => open, setOpen}
									openOnHover
									pinOnClick
									submenuSafe
									placement="right-start"
									closeFocus={null}
									class="h-full"
									menuClass="w-56"
									maxHeight={600}
								>
									{#snippet triggr({ trigger })}
										<MeltButton
											meltElement={trigger}
											title={toggleLabel}
											class="w-full h-full min-w-0 flex items-center gap-2 px-2 text-left"
										>
											{@render rowContent()}
										</MeltButton>
									{/snippet}
									{#snippet children({ item, builders })}
										{@const menu = { item, builders } as SectionMenu}
										<!-- A session or link picked here leaves the menu with the page it opened:
										     only the rows given `menu.item` are melt items that close it themselves. -->
										<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
										<div
											class="flex flex-col gap-0.5 px-1"
											onclick={(e) => {
												if ((e.target as Element).closest('a, [role="option"]')) setOpen(false)
											}}
										>
											<!-- The collapsed rail shows no title on the row, so its menu does. -->
											{#if isCollapsed}
												<div class="flex flex-row items-center h-7 pl-2">
													<span
														class="flex-1 truncate text-secondary text-[0.5rem] font-normal uppercase"
														>{title}</span
													>
												</div>
											{/if}
											{@render actions?.({ inPopover: true, menu })}
											{@render items(true, menu)}
										</div>
									{/snippet}
								</Menu>
							{/snippet}
						</Menubar>
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
				{#if !isCollapsed}
					<!-- Outside the row's control, which is a different element folded and unfolded (melt's
					     inline-block trigger, or a Button): the title stays one element, placed the same. -->
					<span
						class="pointer-events-none absolute inset-y-0 left-2 right-7 flex items-center gap-2"
					>
						<span class="truncate uppercase text-[0.5rem] leading-3 font-normal text-secondary"
							>{title}</span
						>
						{#if notify && (isFolded || disabled)}
							{@render notifyDot('flex-shrink-0')}
						{/if}
					</span>
				{/if}
			</div>
			{#if !isCollapsed && !isFolded}
				{@render actions?.({ inPopover: false })}
			{/if}
		</div>
		{#if !isFolded}
			<div transition:slide={{ duration: 180 }}>
				{@render items(false, undefined)}
			</div>
		{/if}
	</div>
</div>
