<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { classNames } from '$lib/utils'
	import CloseButton from '../CloseButton.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		title?: string | undefined
		overflow_y?: boolean
		noPadding?: boolean
		forceOverflowVisible?: boolean
		tooltip?: string
		documentationLink?: string | undefined
		CloseIcon?: any | undefined
		fullScreen?: boolean
		actions?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		aiId,
		aiDescription,
		title = undefined,
		overflow_y = true,
		noPadding = false,
		forceOverflowVisible = false,
		tooltip = '',
		documentationLink = undefined,
		CloseIcon = undefined,
		fullScreen = true,
		actions,
		children
	}: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<div class={classNames('flex flex-col divide-y', fullScreen ? 'h-screen max-h-screen' : 'h-full')}>
	<div class="flex justify-between w-full items-center px-4 py-2 gap-2">
		<div class="flex items-center gap-2 w-full truncate">
			<div
				use:triggerableByAI={{
					id: `close-${aiId}`,
					description: `Close ${aiDescription}`,
					callback: () => {
						dispatch('close')
					}
				}}
			>
				<CloseButton on:close Icon={CloseIcon} />
			</div>
			<span class="font-semibold truncate text-primary !text-lg max-w-sm"
				>{title ?? ''}
				{#if tooltip != '' || documentationLink}
					<Tooltip {documentationLink}>{tooltip}</Tooltip>
				{/if}</span
			>
		</div>
		{#if actions}
			<div class="flex gap-2 items-center justify-end">
				{@render actions?.()}
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
		{@render children?.()}
	</div>
</div>
