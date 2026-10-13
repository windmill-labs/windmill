<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'
	import { createEventDispatcher, getContext, tick, type Snippet } from 'svelte'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import type { TriggerContext } from '../triggers'
	import type { Item } from '$lib/utils'
	import { Bell, BellOff, Calendar } from 'lucide-svelte'
	import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

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
		/** What the rename popover is renaming, when that is not the error handler's kind — an
		 *  agent has no error handler but is still renamed from here. */
		itemKind?: 'flow' | 'script' | 'agent'
		scriptOrFlowPath?: string
		errorHandlerMuted?: boolean | undefined
		labels?: string[] | undefined
		inheritedLabels?: string[] | undefined
		onSaved?: (newPath: string) => void
		children?: import('svelte').Snippet
		trigger_badges?: import('svelte').Snippet
		/** True for the route's own page: its header becomes the page header. A host that renders
		 *  this inside something else — a session's preview panel — leaves it false and gets the
		 *  row below. */
		ownsPageHeader?: boolean
		/** Whether the bar is wide enough to hold every button beside the summary, decided by
		 * the layout from its own width. Not a viewport media query: the same page also renders
		 * inside an AI session's preview panel, where the window is wide and the pane is not. */
		wide?: boolean
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
		itemKind,
		scriptOrFlowPath,
		errorHandlerMuted = $bindable(),
		labels = $bindable(),
		inheritedLabels = undefined,
		onSaved,
		children,
		trigger_badges,
		ownsPageHeader = false,
		wide = true,
		leading_actions
	}: Props = $props()

	const dispatch = createEventDispatcher()

	// A flow and a script are renamed as what their error handler already names them; an agent has
	// no error handler and says so itself.
	const renameKind = $derived(itemKind ?? errorHandlerKind)

	// Two reasons the row can be too tight, and either one collapses it. The host knows when its
	// own box is narrow while the window is not — a session's preview panel — and says so through
	// `wide`. The band knows its own width, which the host cannot see: these buttons share that row
	// with the breadcrumb, and the trail and summary want ~580px against the set's ~530. Unmeasured
	// (0) counts as wide, because the bar measures itself on mount and starting narrow would pop
	// the buttons out of the menu a frame later.
	// Measured on a flow's page: a 1192px bar leaves the last button 4px past the edge, so the row
	// has to fold before that rather than at the ~1150 the two parts add up to on paper.
	const COLLAPSE_BELOW = 1250
	const roomInBar = $derived(pageHeader.barWidth === 0 || pageHeader.barWidth >= COLLAPSE_BELOW)
	const wideRow = $derived(wide && roomInBar)

	const barButtons = $derived(wideRow ? mainButtons : mainButtons.filter((b) => !b.narrow))

	function dropdownHost(btn: MainButton): MainButton | undefined {
		if (typeof btn.narrow !== 'object') return undefined
		const label = btn.narrow.dropdownOf
		return barButtons.find((b) => b.label === label && !b.buttonProps.disabled)
	}

	async function toggleErrorHandler() {
		if (!errorHandlerKind || !scriptOrFlowPath) return
		const next = await toggleWorkspaceErrorHandler(
			$operatingWorkspace,
			errorHandlerKind,
			scriptOrFlowPath,
			errorHandlerMuted
		)
		if (next !== undefined) errorHandlerMuted = next
	}

	const allMenuItems: Item[] = $derived([
		...(wideRow ? [] : mainButtons.filter((b) => b.narrow && !dropdownHost(b))).map((b) => ({
			displayName: b.label,
			description: b.description,
			icon: b.buttonProps.startIcon,
			href: b.buttonProps.href,
			action: b.buttonProps.onClick,
			disabled: b.buttonProps.disabled,
			type: 'action' as const
		})),
		...(wideRow || !errorHandlerKind
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
			separatorTop: i === 0 && !wideRow
		}))
	])

	function dropdownItemsOf(host: MainButton) {
		if (wideRow) return undefined
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
		kind={renameKind}
		compact
	/>
{/snippet}

<!-- The same editor as the summary's, hung off the band's path segment so it opens under the path.
     with the cursor in the path field. `bind:` cannot be spread, so `labels` is bound twice; both
     instances bind the same slot and only one is ever open. -->
{#snippet pathTrigger(pathLabel: Snippet, triggerClass: string)}
	<SummaryPathDisplay
		label={pathLabel}
		{triggerClass}
		focusField="path"
		{summary}
		{path}
		bind:labels
		{inheritedLabels}
		{onSaved}
		kind={renameKind}
	/>
{/snippet}

<!-- The two halves of the row, rendered side by side in the band and in the two groups of the
     row below. One copy each: a button added to one placement belongs in both. -->
{#snippet badges(size: 'sm' | 'md')}
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
			unifiedSize={size}
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
{/snippet}

{#snippet controls(size: 'sm' | 'md')}
	{@render leading_actions?.()}
	{#if allMenuItems.length > 0}
		{#key allMenuItems}
			<DropdownV2 items={allMenuItems} placement="bottom-end" {size} />
		{/key}
	{/if}
	{#if wideRow && errorHandlerKind && scriptOrFlowPath}
		<ErrorHandlerToggleButton
			kind={errorHandlerKind}
			{scriptOrFlowPath}
			bind:errorHandlerMuted
			unifiedSize={size}
		/>
	{/if}
	{#each barButtons as btn (btn.label)}
		{@const dropdownItems = dropdownItemsOf(btn)}
		<Button
			{...btn.buttonProps}
			startIcon={{ icon: btn.buttonProps.startIcon }}
			unifiedSize={size}
			{dropdownItems}
			dropdownWidth={dropdownItems?.some((i) => i.description) ? 288 : undefined}
			btnClasses="flex items-center gap-1 whitespace-nowrap"
		>
			{btn.label}
		</Button>
	{/each}
{/snippet}

{#snippet actions()}
	{@render badges('sm')}
	{@render controls('sm')}
{/snippet}

{#if ownsPageHeader}
	<!-- The summary keeps its rename-and-labels popover here rather than becoming plain text in
	     the breadcrumb: renaming a script or flow is done from this page, not from the trail. -->
	<PageHeaderContent
		item={{
			kind: errorHandlerKind,
			path,
			summaryContent,
			// Only where this page can save one: without `onSaved` the editor behind it has nothing
			// to write, and the band keeps its own segment, which copies the path.
			pathTrigger: onSaved ? pathTrigger : undefined
		}}
		{actions}
		contexts={headerContexts}
		separator="always"
	/>
{:else}
	<!-- Nested in a session's preview panel, which has a band of its own above it naming the
	     session. A page in there draws its own row instead: registering would rename the band to
	     whatever the panel happens to be showing. -->
	<div class="border-b">
		<div class="mx-auto">
			<div
				class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-12 py-2 md:py-0"
			>
				<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
					<div class="min-w-0">
						<SummaryPathDisplay
							{summary}
							{path}
							bind:labels
							{inheritedLabels}
							{onSaved}
							kind={renameKind}
						/>
					</div>
					{@render badges('md')}
				</div>
				<div class="flex gap-1 items-center pr-4">
					{@render controls('md')}
				</div>
			</div>
		</div>
	</div>
{/if}
