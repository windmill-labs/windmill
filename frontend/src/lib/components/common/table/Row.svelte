<script lang="ts">
	import Star from '$lib/components/Star.svelte'
	import { createEventDispatcher } from 'svelte'
	import RowIcon from './RowIcon.svelte'

	export let marked: string | undefined
	export let starred: boolean
	export let canFavorite: boolean = true

	const dispatch = createEventDispatcher()

	export let kind: 'script' | 'flow' | 'app' | 'raw_app' = 'script'
	export let summary: string | undefined = undefined
	export let path: string
	export let href: string
	export let workspaceId: string
	const color = {
		script: 'hover:bg-blue-50 hover:border-blue-200',
		flow: 'hover:bg-[#f0fdfa] hover:border-[#99f6e4]',
		app: 'hover:bg-[#fff7ed] hover:border-orange-300'
	}[kind]
</script>

<div
	class="hover:bg-surface-hover w-full inline-flex items-center gap-4 first-of-type:!border-t-0
first-of-type:rounded-t-md last-of-type:rounded-b-md {color} [*:not(:last-child)]:border-b px-4 py-2.5"
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
					{!summary || summary.length == 0 ? path : summary}
				{/if}
			</div>
			<div class="text-secondary text-2xs truncate text-left font-light">
				{path}
			</div>
		</div>
	</a>
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
		<div class="w-9" />
	{/if}

	<div class="flex gap-1 items-center justify-end pr-2">
		<slot name="actions" />
	</div>
</div>
