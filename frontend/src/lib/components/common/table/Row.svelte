<script lang="ts">
	import Star from '$lib/components/Star.svelte'
	import { createEventDispatcher } from 'svelte'
	import RowIcon from './RowIcon.svelte'
	import { BellOff } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$lib/navigation'
	import TriggerableByAI from '$lib/components/TriggerableByAI.svelte'

	export let marked: string | undefined
	export let starred: boolean
	export let canFavorite: boolean = true
	export let errorHandlerMuted: boolean = false

	const dispatch = createEventDispatcher()

	export let aiId: string | undefined = undefined
	export let aiDescription: string | undefined = undefined
	export let kind: 'script' | 'flow' | 'app' | 'raw_app' = 'script'
	export let summary: string | undefined = undefined
	export let path: string
	export let href: string
	export let workspaceId: string
	export let depth: number = 0

	let displayPath: string = (depth === 0 ? path : path?.split('/')?.slice(-1)?.[0]) ?? ''
</script>

<TriggerableByAI
	id={aiId}
	description={aiDescription}
	onTrigger={() => {
		goto(href)
	}}
/>
<div
	class={twMerge(
		'hover:bg-surface-hover w-full inline-flex items-center gap-4 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b px-4 py-2.5 border-b last:border-b-0',
		depth > 0 ? '!rounded-none' : ''
	)}
	style={depth > 0 ? `padding-left: ${depth * 32}px;` : ''}
>
	<a {href} class="min-w-0 grow hover:underline decoration-gray-400 inline-flex items-center gap-4">
		<div class="shrink">
			<RowIcon {href} {kind} />
		</div>
		<div class="grow">
			<div class="text-primary flex-wrap text-left text-sm font-semibold">
				{#if marked}
					{@html marked}
				{:else}
					{!summary || summary.length == 0 ? displayPath : summary}
				{/if}
			</div>
			<div class="text-secondary text-2xs truncate text-left font-light">
				{path}
			</div>
		</div>
	</a>

	{#if errorHandlerMuted}
		<BellOff class="w-8 opacity-60" size={12} fill="currentcolor" />
	{/if}

	{#if $$slots.badges}
		<div class="hidden lg:flex flex-row gap-4 items-center">
			<slot name="badges" />
		</div>
	{/if}

	{#if canFavorite}
		<div class="center-center h-full text-sm font-semibold text-secondary">
			<Star
				{kind}
				{path}
				{starred}
				workspace_id={workspaceId}
				on:starred={() => {
					dispatch('change')
				}}
			/>
		</div>
	{:else}
		<div class="w-9"></div>
	{/if}

	<div class="flex gap-1 items-center justify-end pr-2">
		<slot name="actions" />
	</div>
</div>
