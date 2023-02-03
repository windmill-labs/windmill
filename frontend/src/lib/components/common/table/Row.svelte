<script lang="ts">
	import Star from '$lib/components/Star.svelte'
	import { createEventDispatcher } from 'svelte'
	import RowIcon from './RowIcon.svelte'

	export let marked: string | undefined
	export let starred: boolean
	export let canFavorite: boolean = true

	const dispatch = createEventDispatcher()

	export let kind: 'script' | 'flow' | 'app' = 'script'
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
	class="hover:bg-gray-50 w-full inline-flex items-center p-4 gap-4 first-of-type:!border-t-0 
first-of-type:rounded-t-md last-of-type:rounded-b-md {color}"
>
	<RowIcon {href} {kind} />

	<a {href} class="min-w-0 grow hover:underline decoration-gray-400">
		<div class="text-gray-900 flex-wrap text-left text-md font-semibold mb-1 truncate ">
			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="text-gray-600 text-xs truncate text-left font-light">
			{path}
		</div>
	</a>
	{#if $$slots.badges}
		<div class="hidden md:flex flex-row gap-1 items-start flex-wrap">
			<slot name="badges" />
		</div>
	{/if}

	<div class="flex gap-1 items-center justify-end">
		<slot name="actions" />
	</div>
	{#if canFavorite}
		<div class="center-center h-full text-sm font-semibold text-gray-900">
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
	{/if}
</div>
