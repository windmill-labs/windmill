<script lang="ts">
	import { untrack } from 'svelte'
	import Star from '$lib/components/Star.svelte'
	import RowIcon from './RowIcon.svelte'
	import { BellOff, Star as StarIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$lib/navigation'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Tooltip from '../../meltComponents/Tooltip.svelte'
	import Checkbox from '../checkbox/Checkbox.svelte'
	import type { RowSelection } from './rowSelection'
	import RunnableActivityCells from '$lib/components/home/RunnableActivityCells.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { displayDate } from '$lib/utils'
	import { setCompactBadges } from '../badge/density'
	import {
		EMPTY_CELL_DASH,
		HOME_TABLE_BADGE_GRID,
		HOME_TABLE_GRID,
		HOME_TABLE_ROW_SEPARATOR,
		isHomeTable
	} from '$lib/components/home/homeTable'

	interface Props {
		marked: string | undefined
		selected?: boolean
		/** Highlighted by the list's keyboard arrow-navigation (distinct from `selected`,
		 * which is the checkbox multi-select state). Scrolls itself into view. */
		keyboardSelected?: boolean
		disabled?: boolean
		canFavorite?: boolean
		isSelectable?: boolean
		/** When the row is not selectable, render a disabled checkbox with this
		 * reason as a hover tooltip (instead of an empty slot) — explains why the
		 * row can't be selected without greying the whole row via `disabled`. */
		selectDisabledReason?: string
		/** When true, clicking anywhere on the row card (except interactive
		 * children — checkbox, buttons, links) toggles selection. Opt-in so
		 * existing tables that don't want it are unaffected. */
		selectOnRowClick?: boolean
		/** Home-style multi-select: the kind icon doubles as the checkbox instead
		 * of adding a column, so an unused selection costs the row nothing. */
		rowSelection?: RowSelection
		alignWithSelectable?: boolean
		errorHandlerMuted?: boolean
		aiId?: string | undefined
		aiDescription?: string | undefined
		kind?:
			| 'script'
			| 'flow'
			| 'app'
			| 'raw_app'
			| 'agent'
			| 'resource'
			| 'variable'
			| 'resource_type'
			| 'folder'
			| 'schedule'
			| 'trigger'
			| 'http_trigger'
			| 'websocket_trigger'
			| 'kafka_trigger'
			| 'nats_trigger'
			| 'postgres_trigger'
			| 'mqtt_trigger'
			| 'amqp_trigger'
			| 'sqs_trigger'
			| 'gcp_trigger'
			| 'azure_trigger'
			| 'email_trigger'
			| 'data_pipeline'
			| 'datatable_migration'
		triggerKind?: string | undefined
		summary?: string | undefined
		path: string
		href?: string
		workspaceId: string
		depth?: number
		badges?: import('svelte').Snippet
		/** Badge groups the home table gives a column each; elsewhere they render
		 * inline, in this order, after `badges`. */
		/** Appended to the path line, e.g. a script's language. */
		pathSuffix?: string | undefined
		tagBadges?: import('svelte').Snippet
		draftBadge?: import('svelte').Snippet
		sharedBadge?: import('svelte').Snippet
		labelBadges?: import('svelte').Snippet
		/** When the item was last edited, for the home table's last edited column. */
		editedAt?: string
		actions?: import('svelte').Snippet
		customSummary?: import('svelte').Snippet
		/** Rendered inline right after the title, unlike `badges`, which sit in
		 * their own column and are hidden below `lg`. */
		titleBadge?: import('svelte').Snippet
		/** Overrides the secondary path line (e.g. to strike a renamed path).
		 * Falls back to the plain `path` string when not provided. */
		pathDisplay?: import('svelte').Snippet
		onSelect?: (
			e: Event & {
				currentTarget: EventTarget & HTMLInputElement
			}
		) => void
	}

	let {
		marked,
		selected = false,
		keyboardSelected = false,
		disabled = false,
		canFavorite = true,
		isSelectable = false,
		selectDisabledReason = undefined,
		selectOnRowClick = false,
		rowSelection = undefined,
		alignWithSelectable = false,
		errorHandlerMuted = false,
		aiId = undefined,
		aiDescription = undefined,
		kind = 'script',
		triggerKind = undefined,
		summary = undefined,
		path,
		href = undefined,
		workspaceId,
		depth = 0,
		badges,
		pathSuffix = undefined,
		tagBadges,
		draftBadge,
		sharedBadge,
		labelBadges,
		editedAt,
		actions,
		customSummary,
		titleBadge,
		pathDisplay,
		onSelect = () => {}
	}: Props = $props()

	let displayPath: string =
		(untrack(() => depth) === 0
			? untrack(() => path)
			: untrack(() => path)
					?.split('/')
					?.slice(-1)?.[0]) ?? ''

	const homeTable = isHomeTable()
	if (homeTable) setCompactBadges()

	let rowEl: HTMLDivElement | undefined = $state()
	$effect(() => {
		if (keyboardSelected) {
			rowEl?.scrollIntoView({ block: 'nearest' })
		}
	})

	const clickToSelect = $derived(selectOnRowClick && isSelectable && !disabled)
	// Once selection mode is on the whole card toggles, and the title stops being
	// a link so a stray click can't navigate out of the selection.
	const inSelectionMode = $derived(!!rowSelection?.active)

	// Interactive children that handle their own activation — selecting the row on
	// top of them would double-fire (mouse) or hijack their keyboard activation.
	function fromInteractiveChild(e: Event): boolean {
		return !!(e.target as HTMLElement | null)?.closest('a, button, input, [data-row-actions]')
	}

	function handleRowClick(e: MouseEvent) {
		if (inSelectionMode) {
			if (fromInteractiveChild(e)) return
			rowSelection?.onToggle(e)
			return
		}
		if (!clickToSelect) return
		// Don't double-toggle when the click originated from an interactive child
		// (the checkbox itself, action buttons, or the title link).
		if (fromInteractiveChild(e)) return
		onSelect?.(e as unknown as Event & { currentTarget: EventTarget & HTMLInputElement })
	}

	function handleRowKeydown(e: KeyboardEvent) {
		if (!clickToSelect) return
		if (e.key !== 'Enter' && e.key !== ' ') return
		// Same guard as the click path: activating a child (checkbox / action button
		// / title link) via Enter/Space must not also toggle the row's selection.
		if (fromInteractiveChild(e)) return
		e.preventDefault()
		onSelect?.(e as unknown as Event & { currentTarget: EventTarget & HTMLInputElement })
	}
</script>

{#if href}
	<div
		style="display: none"
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				goto(href)
			}
		}}
	></div>
{/if}
{#if homeTable}
	<div
		bind:this={rowEl}
		data-row-selection-key={rowSelection?.key}
		data-row-keyboard-selected={keyboardSelected ? 'true' : undefined}
		class={twMerge(
			HOME_TABLE_GRID,
			'group/row relative w-full pl-5 pr-3 py-2.5',
			HOME_TABLE_ROW_SEPARATOR,
			disabled ? 'opacity-25' : 'hover:bg-surface-hover',
			inSelectionMode ? 'cursor-pointer select-none' : '',
			rowSelection?.selected
				? 'bg-surface-accent-selected'
				: keyboardSelected
					? 'bg-gray-200 dark:bg-gray-700'
					: ''
		)}
		onclick={handleRowClick}
	>
		{#if href && !inSelectionMode}
			<a
				{href}
				aria-label={!summary || summary.length == 0 ? displayPath : summary}
				class="absolute inset-0"
			></a>
		{/if}
		<!-- The kind icon, swapped for the checkbox on hover and in selection mode. In the
		     tree it moves right by the same depth offset as the name, under its folder. -->
		<div
			class="relative z-[1] w-4 h-4"
			style={depth > 0 ? `transform: translateX(${depth * 16}px);` : ''}
		>
			<div
				class={twMerge(
					'absolute inset-0',
					rowSelection ? (rowSelection.active ? 'invisible' : 'group-hover/row:invisible') : ''
				)}
			>
				<RowIcon {kind} {triggerKind} />
			</div>
			{#if rowSelection}
				<Checkbox
					class={twMerge(
						'absolute inset-0 w-4 h-4',
						rowSelection.active ? '' : 'invisible group-hover/row:visible'
					)}
					checked={rowSelection.selected}
					title={rowSelection.selected ? 'Deselect' : 'Select (shift-click to select a range)'}
					onClick={(e) => {
						e.stopPropagation()
						rowSelection?.onToggle(e)
					}}
				/>
			{/if}
		</div>
		<div
			class="min-w-0 flex items-center gap-2"
			style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
		>
			{@render rowContent(false, !!href && !inSelectionMode, false)}
			{#if errorHandlerMuted}
				<BellOff class="shrink-0 opacity-60" size={12} fill="currentcolor" />
			{/if}
		</div>
		<!-- Fixed sub-columns, so each badge group lines up across rows. -->
		<div class="relative z-[1] hidden lg:grid {HOME_TABLE_BADGE_GRID}">
			<div class="flex flex-wrap items-center gap-1 min-w-0 {EMPTY_CELL_DASH}">
				{@render badges?.()}
				{@render labelBadges?.()}
			</div>
			<div class="flex items-center">{@render sharedBadge?.()}</div>
		</div>
		<RunnableActivityCells kind={kind == 'script' || kind == 'flow' ? kind : undefined} {path} />
		<div class="hidden lg:block text-xs text-secondary truncate {EMPTY_CELL_DASH}">
			{#if editedAt}
				<span title={displayDate(editedAt)}><TimeAgo date={editedAt} compact /> ago</span>
			{/if}
		</div>
		<div class="relative z-[1] flex items-center justify-end gap-2">
			<div data-row-actions class="flex gap-2 items-center">
				{@render actions?.()}
			</div>
		</div>
	</div>
{:else}
	<!-- Tree-view alignment: a folder header's icon sits at px-4 (16px) + its inner
	     padding-left of depth*16, i.e. (depth+1)*16. This row's inline padding-left
	     overrides px-4, so it must carry the full (depth+1)*16 for a file to line up
	     with its sibling folder at the same depth. -->
	<div
		bind:this={rowEl}
		data-row-selection-key={rowSelection?.key}
		data-row-keyboard-selected={keyboardSelected ? 'true' : undefined}
		class={twMerge(
			'group/row relative w-full inline-flex items-center gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b pl-4 pr-1 py-3 border-b last:border-b-0',
			depth > 0 ? '!rounded-none' : '',
			disabled ? 'opacity-25' : 'hover:bg-surface-hover',
			clickToSelect || inSelectionMode ? 'cursor-pointer select-none' : '',
			selected || rowSelection?.selected
				? 'bg-surface-accent-selected'
				: keyboardSelected
					? 'bg-gray-200 dark:bg-gray-700'
					: ''
		)}
		style={depth > 0 ? `padding-left: ${(depth + 1) * 16}px;` : ''}
		role={clickToSelect ? 'button' : undefined}
		tabindex={clickToSelect ? 0 : undefined}
		onclick={handleRowClick}
		onkeydown={clickToSelect ? handleRowKeydown : undefined}
	>
		<!-- Everything interactive sits at z-[1], above the title link's row-wide
		     click overlay. -->
		{#if isSelectable}
			<div class="relative z-[1] flex">
				<Checkbox checked={selected} onChange={onSelect} />
			</div>
		{:else if selectDisabledReason}
			<div class="relative z-[1] flex">
				<Tooltip class="cursor-not-allowed">
					<Checkbox disabled checked={false} />
					{#snippet text()}{selectDisabledReason}{/snippet}
				</Tooltip>
			</div>
		{:else if alignWithSelectable}
			<div class="rounded max-w-4 w-full"></div>
		{/if}

		{#if rowSelection}
			<!-- The icon slot itself: the kind icon until the row is hovered (or
			     selection mode is on), the checkbox from then on. Both are stacked in a
			     fixed 16px box and swapped with visibility so nothing shifts. -->
			<div class="shrink relative z-[1] w-4 h-4">
				<div
					class={twMerge(
						'absolute inset-0',
						rowSelection.active ? 'invisible' : 'group-hover/row:invisible'
					)}
				>
					<RowIcon {kind} {triggerKind} />
				</div>
				<Checkbox
					class={twMerge(
						'absolute inset-0 w-4 h-4',
						rowSelection.active ? '' : 'invisible group-hover/row:visible'
					)}
					checked={rowSelection.selected}
					title={rowSelection.selected ? 'Deselect' : 'Select (shift-click to select a range)'}
					onClick={(e) => {
						// Left unprevented on purpose: the browser's own toggle already lands
						// on the value we are about to compute, except on a range re-select,
						// which Checkbox re-asserts. Preventing it would revert the box AFTER
						// the update and leave every clicked row visually unticked.
						e.stopPropagation()
						rowSelection?.onToggle(e)
					}}
				/>
			</div>
		{/if}

		<!-- The link covers the whole row rather than wrapping the title, so the
		     badges on the title line can hold their own popovers and buttons. -->
		{#if href && !inSelectionMode}
			<a
				{href}
				aria-label={!summary || summary.length == 0 ? displayPath : summary}
				class="absolute inset-0"
			></a>
		{/if}
		{@render rowContent(!rowSelection, !!href && !inSelectionMode)}

		{#if errorHandlerMuted}
			<BellOff class="w-8 opacity-60" size={12} fill="currentcolor" />
		{/if}

		<div class="relative z-[1] flex items-center gap-3 shrink-0">
			{#if kind == 'script' || kind == 'flow' || kind == 'app' || kind == 'raw_app' || kind == 'agent'}
				<RunnableActivityCells
					kind={kind == 'script' || kind == 'flow' ? kind : undefined}
					{path}
				/>
			{/if}

			{@render favorite()}

			<div data-row-actions class="flex gap-2 items-center justify-end">
				{@render actions?.()}
			</div>
		</div>
	</div>
{/if}

{#snippet favorite()}
	{#if canFavorite && (kind == 'app' || kind == 'raw_app' || kind == 'script' || kind == 'flow')}
		<div class="center-center h-full text-xs font-semibold text-secondary w-7">
			<Star {kind} {path} {workspaceId} {summary} />
		</div>
	{:else}
		<div class="w-7"></div>
	{/if}
{/snippet}

{#snippet inlineFavorite()}
	{#if kind == 'app' || kind == 'raw_app' || kind == 'script' || kind == 'flow'}
		<div class="relative z-[1] flex items-center shrink-0 -my-1 text-secondary">
			{#if canFavorite}
				<Star {kind} {path} {workspaceId} {summary} size={12} yellowWhenStarred revealOnRowHover />
			{:else}
				<span
					class="p-1 opacity-20 invisible group-hover/row:visible"
					title="Deploy it to add it to favorites"
				>
					<StarIcon size={12} />
				</span>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet rowContent(withIcon: boolean, linked: boolean, inlineBadges = true)}
	{#if withIcon}
		<div class="shrink">
			<RowIcon {kind} {triggerKind} />
		</div>
	{/if}
	<div class="min-w-0 grow">
		<div class="flex items-center gap-1.5 min-w-0">
			<div
				class={twMerge(
					'text-emphasis flex-wrap text-left text-xs min-w-0',
					homeTable ? 'font-normal' : 'font-semibold',
					titleBadge ? 'inline-flex items-center gap-2' : '',
					homeTable && !titleBadge ? 'line-clamp-2 break-words' : '',
					linked ? 'group-hover/row:underline decoration-gray-400' : ''
				)}
				title={homeTable && !customSummary
					? !summary || summary.length == 0
						? displayPath
						: summary
					: undefined}
			>
				{#if customSummary}
					{@render customSummary?.()}
				{:else if marked}
					{@html marked}
				{:else}
					{!summary || summary.length == 0 ? displayPath : summary}
				{/if}
				{@render titleBadge?.()}
			</div>
			{#if !inlineBadges && tagBadges}
				<div class="relative z-[1] flex items-center gap-1 shrink-0">{@render tagBadges()}</div>
			{/if}
			{#if !inlineBadges && draftBadge}
				<!-- The home table keeps the draft marker on the title line. -->
				<div class="relative z-[1] flex items-center shrink-0">{@render draftBadge()}</div>
			{/if}
			{#if homeTable}
				{@render inlineFavorite()}
			{/if}
			{#if inlineBadges && (badges || tagBadges || draftBadge || sharedBadge || labelBadges)}
				<div class="relative z-[1] hidden lg:flex flex-row gap-1 items-center shrink-0">
					{@render badges?.()}
					{@render tagBadges?.()}
					{@render draftBadge?.()}
					{@render sharedBadge?.()}
					{@render labelBadges?.()}
				</div>
			{/if}
		</div>
		<div class="text-hint text-3xs truncate text-left font-normal" title={path}>
			{#if pathDisplay}
				{@render pathDisplay()}
			{:else}
				{path}
			{/if}
			{#if pathSuffix}
				<span class="opacity-75">· {pathSuffix}</span>
			{/if}
		</div>
	</div>
{/snippet}
