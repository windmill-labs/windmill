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
	import { Bell, BellOff, Calendar, UserPlus } from 'lucide-svelte'
	import { toggleWorkspaceErrorHandler } from './errorHandlerToggle'
	import {
		operatingWorkspaceContextEntry,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import OpenInSessionButton, {
		type OpenInSessionSource
	} from '$lib/components/sessions/OpenInSessionButton.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	type MainButton = {
		label: string
		/** Shown under the label once the button is a row of a menu. */
		description?: string
		buttonProps: ButtonProps
		/** A secondary action: a bar button while the bar has room for every one, then, from the
		 * first fold, in the ellipsis menu or in the dropdown of the enabled bar button labelled
		 * `dropdownOf` (the menu when there is none). Unset makes it a main button, which goes to
		 * the menu only on the folds after that. */
		placement?: 'menu' | { dropdownOf: string }
		/** In its `placement` at every width, never a bar button of its own. */
		alwaysPlaced?: boolean
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
	const headerContexts = new Map<any, any>([
		['TriggerContext', triggerContext],
		operatingWorkspaceContextEntry()
	])

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
		/** Marks that belong to the item's name, such as its favorite star: drawn right after the
		 *  summary rather than among the badges and buttons at the far end. */
		name_marks?: import('svelte').Snippet
		/** Opens the item's Share dialog, from a Share button placed right before Edit. */
		onShare?: () => void
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
		leading_actions,
		name_marks,
		onShare
	}: Props = $props()

	const dispatch = createEventDispatcher()

	// A flow and a script are renamed as what their error handler already names them; an agent has
	// no error handler and says so itself.
	const renameKind = $derived(itemKind ?? errorHandlerKind)

	// The session's preview opens on this same deployed page (`/get/`), not the item's editor:
	// someone here came to run the item, not to change it.
	const sessionSource: OpenInSessionSource | undefined = $derived(
		errorHandlerKind && path
			? {
					page: () => `/${errorHandlerKind}s/get/${path}`,
					workspaceId: $operatingWorkspace ?? undefined
				}
			: undefined
	)

	// The band folds the buttons once its breadcrumb has given all it can (`actionFolds` below):
	// first the secondary ones go to their `placement`, then every bar button but the last, which
	// is what the page is for (Edit), then that one too. Outside the band — a session's preview
	// panel — no bar measures the row, but the host knows when its own box is narrow while the
	// window is not and says so through `wide`: the secondary buttons fold there, as at the first
	// fold, and the error handler's toggle goes to the menu.
	const fold = $derived(ownsPageHeader ? pageHeader.actionFold : wide ? 0 : 1)
	const wideRow = $derived(wide && fold <= 1)

	// Share is a main button the header places itself: right before Edit, or last for someone
	// who gets no Edit. It folds like the other main buttons, before Edit does.
	const shareButton: MainButton | undefined = $derived(
		onShare
			? {
					label: 'Share',
					buttonProps: { onClick: onShare, variant: 'default', startIcon: UserPlus }
				}
			: undefined
	)
	const buttons = $derived.by(() => {
		if (!shareButton) return mainButtons
		const edit = mainButtons.findIndex((b) => b.label === 'Edit')
		const at = edit === -1 ? mainButtons.length : edit
		return [...mainButtons.slice(0, at), shareButton, ...mainButtons.slice(at)]
	})

	const rowButtons = $derived(buttons.filter((b) => !b.placement))
	const barButtons = $derived(
		fold === 0
			? buttons.filter((b) => !b.alwaysPlaced)
			: fold === 1
				? rowButtons
				: fold === 2
					? rowButtons.slice(-1)
					: []
	)

	function dropdownHost(btn: MainButton): MainButton | undefined {
		if (typeof btn.placement !== 'object' || barButtons.includes(btn)) return undefined
		const label = btn.placement.dropdownOf
		return barButtons.find((b) => b.label === label && !b.buttonProps.disabled)
	}

	/** What the menu holds of the buttons: the folded bar buttons, each followed by what its
	 *  dropdown would have held, then the secondary actions with no bar button to hang from. */
	const menuButtons = $derived.by(() => {
		const folded = rowButtons.filter((b) => !barButtons.includes(b))
		const homeless = buttons.filter(
			(b) => b.placement && !barButtons.includes(b) && !dropdownHost(b)
		)
		const childOf = (host: MainButton) =>
			homeless.filter(
				(b) => typeof b.placement === 'object' && b.placement.dropdownOf === host.label
			)
		const placed = new Set(folded.flatMap(childOf))
		return [
			...folded.flatMap((host) => [host, ...childOf(host)]),
			...homeless.filter((b) => !placed.has(b))
		]
	})

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
		...menuButtons.map((b) => ({
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
			separatorTop: i === 0 && (menuButtons.length > 0 || (!wideRow && !!errorHandlerKind))
		}))
	])

	// The session button goes right before the page's main action, the last bar button (Edit), and
	// before Share too when Share sits beside it, so the two stay together.
	const sessionAt = $derived(
		barButtons.length - (barButtons.length >= 2 && barButtons.at(-2) === shareButton ? 2 : 1)
	)

	function dropdownItemsOf(host: MainButton) {
		const items = buttons
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
	<div class="flex items-center gap-1 min-w-0">
		<SummaryPathDisplay
			{summary}
			{path}
			bind:labels
			{inheritedLabels}
			{onSaved}
			kind={renameKind}
			compact
		/>
		{#if name_marks}<span class="flex items-center shrink-0">{@render name_marks()}</span>{/if}
	</div>
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
	{#if ownsPageHeader && barButtons.length === 0}
		<OpenInSessionButton source={sessionSource} />
	{/if}
	{#each barButtons as btn, i (btn.label)}
		{@const dropdownItems = dropdownItemsOf(btn)}
		{#if ownsPageHeader && i === sessionAt}
			<OpenInSessionButton source={sessionSource} />
		{/if}
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
			// Read-only, the summary control falls back to the path when there is no summary, so the
			// path is what names the item there and the trail's own copy can go.
			summary: onSaved ? summary : summary || path,
			summaryContent,
			// Only where this page can save one: without `onSaved` the editor behind it has nothing
			// to write, and the band keeps its own segment, which copies the path.
			pathTrigger: onSaved ? pathTrigger : undefined
		}}
		{actions}
		{sessionSource}
		actionFolds={3}
		earlyActionFolds={1}
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
					{@render name_marks?.()}
					{@render badges('md')}
				</div>
				<div class="flex gap-1 items-center pr-4">
					{@render controls('md')}
				</div>
			</div>
		</div>
	</div>
{/if}
