<script lang="ts">
	import Star from '$lib/components/Star.svelte'
	import RowIcon from './RowIcon.svelte'
	import { BellOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$lib/navigation'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	interface Props {
		marked: string | undefined
		selected?: boolean
		disabled?: boolean
		canFavorite?: boolean
		isSelectable?: boolean
		alignWithSelectable?: boolean
		errorHandlerMuted?: boolean
		aiId?: string | undefined
		aiDescription?: string | undefined
		kind?: 'script' | 'flow' | 'app' | 'raw_app' | 'resource' | 'variable' | 'resource_type' | 'folder' | 'schedule' | 'trigger'
		triggerKind?: string | undefined
		summary?: string | undefined
		path: string
		href?: string
		workspaceId: string
		depth?: number
		badges?: import('svelte').Snippet
		actions?: import('svelte').Snippet
		customSummary?: import('svelte').Snippet
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
		onSelect = () => {}
	}: Props = $props()

	let displayPath: string = (depth === 0 ? path : path?.split('/')?.slice(-1)?.[0]) ?? ''
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
		selected ? 'bg-surface-accent-selected' : ''
	)}
	style={depth > 0 ? `padding-left: ${depth * 32}px;` : ''}
>
	{#if isSelectable}
		<input type="checkbox" checked={selected} onchange={onSelect} class="rounded max-w-4 w-full" />
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

	<div class="flex gap-1 items-center justify-end pr-2">
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
		<div class="text-hint text-3xs truncate text-left font-normal">
			{path}
		</div>
	</div>
{/snippet}
