<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import type { TriggerContext } from '../triggers'
	import type { Item } from '$lib/utils'
	import { Bell, BellOff, Calendar } from 'lucide-svelte'
	import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'

	type MainButton = {
		label: string
		/** Shown under the label once the button has collapsed into a menu. */
		description?: string
		buttonProps: ButtonProps
		/** Where the button lands below the `lg` breakpoint, where the bar cannot hold every
		 * button beside the summary: the ellipsis menu, or the dropdown of the enabled bar button
		 * labelled `dropdownOf` (the menu when there is none). Unset keeps it in the bar at every
		 * width. */
		narrow?: 'menu' | { dropdownOf: string }
	}

	type ButtonProps = any
	type MenuItemButton = {
		label: string
		Icon: any
		onclick: (e: MouseEvent) => void
		color?: 'red'
		disabled?: boolean
	}

	// An agent's page has no triggers, and so no context for them.
	const triggerContext = getContext<TriggerContext | undefined>('TriggerContext')
	const { triggersCount, triggersState } = $state(triggerContext ?? ({} as Partial<TriggerContext>))
	// The buttons below render in the page header, away from this page's tree.
	const headerContexts = new Map<any, any>([['TriggerContext', triggerContext]])

	interface Props {
		mainButtons?: MainButton[]
		menuItems?: MenuItemButton[]
		summary?: string
		path?: string
		tag?: string | undefined
		/** Unset for what has no workspace error handler to mute, such as an agent. */
		errorHandlerKind?: 'flow' | 'script'
		scriptOrFlowPath?: string
		errorHandlerMuted?: boolean | undefined
		labels?: string[] | undefined
		inheritedLabels?: string[] | undefined
		onSaved?: (newPath: string) => void
		children?: import('svelte').Snippet
		trigger_badges?: import('svelte').Snippet
		/** Controls ahead of the menu, such as the way an agent's page runs it. */
		leading_actions?: import('svelte').Snippet
	}

	let {
		mainButtons = [],
		menuItems = [],
		summary,
		path,
		tag,
		errorHandlerKind,
		scriptOrFlowPath,
		errorHandlerMuted = $bindable(),
		labels = $bindable(),
		inheritedLabels = undefined,
		onSaved,
		children,
		trigger_badges,
		leading_actions
	}: Props = $props()

	const dispatch = createEventDispatcher()

	// These buttons share the page header's row with the breadcrumb, so what decides whether they
	// all fit is that row's width, not the window's — the sidebar and a page's side panel both take
	// from it. The trail and the summary want ~560px and the full set is ~530px wide. Unmeasured (0)
	// counts as wide: the bar measures itself on mount, and starting narrow would pop the buttons
	// out of the menu a frame later.
	const COLLAPSE_BELOW = 1150
	const wide = $derived(pageHeader.barWidth === 0 || pageHeader.barWidth >= COLLAPSE_BELOW)

	const barButtons = $derived(wide ? mainButtons : mainButtons.filter((b) => !b.narrow))

	function dropdownHost(btn: MainButton): MainButton | undefined {
		if (typeof btn.narrow !== 'object') return undefined
		const label = btn.narrow.dropdownOf
		return barButtons.find((b) => b.label === label && !b.buttonProps.disabled)
	}

	async function toggleErrorHandler() {
		if (!errorHandlerKind || !scriptOrFlowPath) return
		const next = await toggleWorkspaceErrorHandler(
			errorHandlerKind,
			scriptOrFlowPath,
			errorHandlerMuted
		)
		if (next !== undefined) errorHandlerMuted = next
	}

	const allMenuItems: Item[] = $derived([
		...(wide ? [] : mainButtons.filter((b) => b.narrow && !dropdownHost(b))).map((b) => ({
			displayName: b.label,
			description: b.description,
			icon: b.buttonProps.startIcon,
			href: b.buttonProps.href,
			action: b.buttonProps.onClick,
			disabled: b.buttonProps.disabled,
			type: 'action' as const
		})),
		...(wide || !errorHandlerKind
			? []
			: [
					{
						displayName: errorHandlerMuted ? 'Unmute error handler' : 'Mute error handler',
						icon: errorHandlerMuted ? BellOff : Bell,
						action: toggleErrorHandler,
						type: 'action' as const
					}
				]),
		...menuItems.map((item, i) => ({
			displayName: item.label,
			icon: item.Icon,
			action: item.onclick,
			disabled: item.disabled,
			type: item.color === 'red' ? ('delete' as const) : ('action' as const),
			separatorTop: i === 0 && !wide
		}))
	])

	function dropdownItemsOf(host: MainButton) {
		if (wide) return undefined
		const items = mainButtons
			.filter((b) => dropdownHost(b) === host)
			.map((b) => ({
				label: b.label,
				description: b.description,
				icon: b.buttonProps.startIcon,
				href: b.buttonProps.href,
				onClick: b.buttonProps.onClick,
				disabled: b.buttonProps.disabled
			}))
		return items.length > 0 ? items : undefined
	}
</script>

{#snippet summaryContent()}
	<SummaryPathDisplay
		{summary}
		{path}
		bind:labels
		{inheritedLabels}
		{onSaved}
		kind={errorHandlerKind}
		compact
	/>
{/snippet}

{#snippet actions()}
	{#if tag}
		<Badge>tag: {tag}</Badge>
	{/if}
	{@render children?.()}
	{#if triggersState?.triggers?.some((t) => t.isPrimary && !t.isDraft)}
		{@const primarySchedule = triggersState.triggers.findIndex((t) => t.isPrimary && !t.isDraft)}
		<Button
			btnClasses="inline-flex"
			startIcon={{ icon: Calendar }}
			variant="default"
			unifiedSize="sm"
			on:click={async () => {
				dispatch('seeTriggers')
				await tick()
				triggersState.selectedTriggerIndex = primarySchedule
			}}
		>
			{$triggersCount?.primary_schedule?.schedule ?? ''}
		</Button>
	{/if}
	{@render trigger_badges?.()}
	{@render leading_actions?.()}
	{#if allMenuItems.length > 0}
		{#key allMenuItems}
			<DropdownV2 items={allMenuItems} placement="bottom-end" size="sm" />
		{/key}
	{/if}
	{#if wide && errorHandlerKind && scriptOrFlowPath}
		<ErrorHandlerToggleButton
			kind={errorHandlerKind}
			{scriptOrFlowPath}
			bind:errorHandlerMuted
			unifiedSize="sm"
		/>
	{/if}
	{#each barButtons as btn (btn.label)}
		{@const dropdownItems = dropdownItemsOf(btn)}
		<Button
			{...btn.buttonProps}
			startIcon={{ icon: btn.buttonProps.startIcon }}
			unifiedSize="sm"
			{dropdownItems}
			dropdownWidth={dropdownItems?.some((i) => i.description) ? 288 : undefined}
			btnClasses="flex items-center gap-1 whitespace-nowrap"
		>
			{btn.label}
		</Button>
	{/each}
{/snippet}

<!-- The summary keeps its rename-and-labels popover here rather than becoming plain text in
     the breadcrumb: renaming a script or flow is done from this page, not from the trail. -->
<PageHeaderContent
	item={{ kind: errorHandlerKind, path, summaryContent }}
	{actions}
	contexts={headerContexts}
/>
