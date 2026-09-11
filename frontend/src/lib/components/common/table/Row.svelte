<script lang="ts">
	import { untrack } from 'svelte'
	import Star from '$lib/components/Star.svelte'
	import RowIcon from './RowIcon.svelte'
	import { BellOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$lib/navigation'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import Tooltip from '../../meltComponents/Tooltip.svelte'
	import Checkbox from '../checkbox/Checkbox.svelte'
	import type { RowSelection } from './rowSelection'

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
		actions?: import('svelte').Snippet
		customSummary?: import('svelte').Snippet
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
		actions,
		customSummary,
		pathDisplay,
		onSelect = () => {}
	}: Props = $props()

	let displayPath: string =
		(untrack(() => depth) === 0
			? untrack(() => path)
			: untrack(() => path)
					?.split('/')
					?.slice(-1)?.[0]) ?? ''

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
<!-- Tree-view alignment: a folder header's icon sits at px-4 (16px) + its inner
     padding-left of depth*16, i.e. (depth+1)*16. This row's inline padding-left
     overrides px-4, so it must carry the full (depth+1)*16 for a file to line up
     with its sibling folder at the same depth. -->
<div
	bind:this={rowEl}
	data-row-selection-key={rowSelection?.key}
	data-row-keyboard-selected={keyboardSelected ? 'true' : undefined}
	class={twMerge(
		'group/row w-full inline-flex items-center gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b px-4 py-3 border-b last:border-b-0',
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
	{#if isSelectable}
		<Checkbox checked={selected} onChange={onSelect} />
	{:else if selectDisabledReason}
		<Tooltip class="cursor-not-allowed">
			<Checkbox disabled checked={false} />
			{#snippet text()}{selectDisabledReason}{/snippet}
		</Tooltip>
	{:else if alignWithSelectable}
		<div class="rounded max-w-4 w-full"></div>
	{/if}

	{#if rowSelection}
		<!-- The icon slot itself: the kind icon until the row is hovered (or
		     selection mode is on), the checkbox from then on. Both are stacked in a
		     fixed 16px box and swapped with visibility so nothing shifts. -->
		<div class="shrink relative w-4 h-4">
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

	{#if href && !inSelectionMode}
		<a
			{href}
			class="min-w-0 grow hover:underline decoration-gray-400 inline-flex items-center gap-4"
		>
			{@render rowContent(!rowSelection)}
		</a>
	{:else}
		{@render rowContent(!rowSelection)}
	{/if}

	{#if errorHandlerMuted}
		<BellOff class="w-8 opacity-60" size={12} fill="currentcolor" />
	{/if}

	{#if badges}
		<div class="hidden lg:flex flex-row gap-4 items-center">
			{@render badges?.()}
		</div>
	{/if}

	{#if canFavorite && (kind == 'app' || kind == 'raw_app' || kind == 'script' || kind == 'flow')}
		<div class="center-center h-full text-xs font-semibold text-secondary w-9">
			<Star {kind} {path} {workspaceId} {summary} />
		</div>
	{:else}
		<div class="w-9"></div>
	{/if}

	<div data-row-actions class="flex gap-1 items-center justify-end pr-2">
		{@render actions?.()}
	</div>
</div>

{#snippet rowContent(withIcon: boolean)}
	{#if withIcon}
		<div class="shrink">
			<RowIcon {kind} {triggerKind} />
		</div>
	{/if}
	<div class="grow min-w-0">
		<div class="text-emphasis flex-wrap text-left text-xs font-semibold">
			{#if customSummary}
				{@render customSummary?.()}
			{:else if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? displayPath : summary}
			{/if}
		</div>
		<div class="text-hint text-3xs truncate text-left font-normal" title={path}>
			{#if pathDisplay}
				{@render pathDisplay()}
			{:else}
				{path}
			{/if}
		</div>
	</div>
{/snippet}
