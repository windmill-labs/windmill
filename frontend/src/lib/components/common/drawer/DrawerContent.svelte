<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { classNames } from '$lib/utils'
	import CloseButton from '../CloseButton.svelte'

	interface Props {
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
</script>

<div class={classNames('flex flex-col divide-y', fullScreen ? 'h-screen max-h-screen' : 'h-full')}>
	<div class="flex justify-between w-full items-center px-4 py-2 gap-2">
		<div class="flex items-center gap-2 w-full truncate">
			<CloseButton on:close Icon={CloseIcon} />

			<span class="font-semibold truncate text-primary !text-lg max-w-sm"
				>{title ?? ''}
				{#if tooltip != '' || documentationLink}
					<Tooltip {documentationLink} scale={0.9}>{tooltip}</Tooltip>
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
