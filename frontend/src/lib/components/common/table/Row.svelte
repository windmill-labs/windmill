<script lang="ts">
	import Star from '$lib/components/Star.svelte'

	import { createEventDispatcher } from 'svelte'

	import RowIcon from './RowIcon.svelte'

	export let marked: string | undefined
	export let starred: boolean

	const dispatch = createEventDispatcher()

	export let kind: 'script' | 'flow' | 'app' = 'script'
	export let summary: string | undefined = undefined
	export let path: string
	export let href: string
	export let workspaceId: string
</script>

<a class="hover:bg-gray-50 cursor-pointer w-full flex items-center p-4 gap-4" {href}>
	<RowIcon {kind} />

	<div class="w-full">
		<div class="text-gray-900 flex-wrap text-md font-semibold mb-1">
			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="text-gray-600 text-xs ">
			{path}
		</div>
	</div>
	<div class="w-96 flex flex-row max-w-xs gap-1 items-start flex-wrap">
		<slot name="badges" />
	</div>

	<div class="flex gap-1 items-center justify-end">
		<slot name="actions" />
	</div>
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
</a>
