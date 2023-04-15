<script lang="ts">
	import { classNames } from '$lib/utils'
	import CloseButton from '../CloseButton.svelte'

	export let title: string | undefined = undefined
	export let overflow_y = true
	export let noPadding = false
	export let forceOverflowVisible = false
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between w-wull items-center px-2 py-2">
		<div class="flex items-center gap-2">
			<CloseButton on:close />

			<span class="font-semibold truncate text-gray-800">{title ?? ''}</span>
		</div>
		{#if $$slots.actions}
			<div class="flex gap-2 items-center justify-end">
				<slot name="actions" />
			</div>
		{/if}
	</div>

	<div
		class={classNames(
			noPadding ? '' : 'p-4',
			'grow h-full max-h-full',
			forceOverflowVisible ? '!overflow-visible' : ''
		)}
		class:overflow-y-auto={overflow_y}
	>
		<slot />
	</div>
</div>
