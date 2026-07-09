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

	interface Props {
		marked: string | undefined
		selected?: boolean
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
		disabled = false,
		canFavorite = true,
		isSelectable = false,
		selectDisabledReason = undefined,
		selectOnRowClick = false,
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


	const clickToSelect = $derived(selectOnRowClick && isSelectable && !disabled)

	// Interactive children that handle their own activation — selecting the row on
	// top of them would double-fire (mouse) or hijack their keyboard activation.
	function fromInteractiveChild(e: Event): boolean {
		return !!(e.target as HTMLElement | null)?.closest('a, button, input, [data-row-actions]')
	}

	function handleRowClick(e: MouseEvent) {
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
<div
	class={twMerge(
		'w-full inline-flex items-center gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b px-4 py-3 border-b last:border-b-0',
		depth > 0 ? '!rounded-none' : '',
		disabled ? 'opacity-25' : 'hover:bg-surface-hover',
		clickToSelect ? 'cursor-pointer select-none' : '',
		selected ? 'bg-surface-accent-selected' : ''
	)}
	style={depth > 0 ? `padding-left: ${depth * 32}px;` : ''}
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

	{#if href}
		<a
			{href}
			class="min-w-0 grow hover:underline decoration-gray-400 inline-flex items-center gap-4"
		>
			{@render rowContent()}
		</a>
	{:else}
		{@render rowContent()}
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

{#snippet rowContent()}
	<div class="shrink">
		<RowIcon {kind} {triggerKind} />
	</div>
	<div class="grow">
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
