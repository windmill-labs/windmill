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

<a
	class="hover:bg-gray-50 cursor-pointer w-full flex items-center p-4 gap-4 {color} rounded-md"
	{href}
>
	<RowIcon {kind} />

	<div class="w-full min-w-0 ">
		<div class="text-gray-900 flex-wrap text-md font-semibold mb-1 truncate">
			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="text-gray-600 text-xs truncate">
			{path}
		</div>
	</div>
	<div class="w-96 hidden lg:flex flex-row max-w-xs gap-1 items-start flex-wrap">
		<slot name="badges" />
	</div>

	<div class="flex gap-1 items-center justify-end">
		<slot name="actions" />
	</div>
	{#if canFavorite}
		<div class="text-left text-sm font-semibold text-gray-900">
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
</a>
