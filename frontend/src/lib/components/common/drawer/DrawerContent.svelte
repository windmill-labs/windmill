<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { classNames } from '$lib/utils'
	import CloseButton from '../CloseButton.svelte'

	export let title: string | undefined = undefined
	export let overflow_y = true
	export let noPadding = false
	export let forceOverflowVisible = false
	export let tooltip: string = ''
	export let documentationLink: string | undefined = undefined
</script>

<div class="flex flex-col divide-y h-screen max-h-screen">
	<div class="flex justify-between w-full items-center px-2 py-2 gap-2">
		<div class="flex items-center gap-2 min-w-0">
			<CloseButton on:close />

			<span class="font-semibold truncate text-gray-800"
				>{title ?? ''}
				{#if tooltip != '' || documentationLink}
					<Tooltip {documentationLink} scale={0.9}>{tooltip}</Tooltip>
				{/if}</span
			>
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
