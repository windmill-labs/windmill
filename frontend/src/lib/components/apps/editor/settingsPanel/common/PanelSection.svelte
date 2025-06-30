<script lang="ts">
	import { classNames } from '$lib/utils'
	import Tooltip from '../../../../Tooltip.svelte'

	interface Props {
		title: string
		noPadding?: boolean
		fullHeight?: boolean
		titlePadding?: string
		tooltip?: string
		documentationLink?: string | undefined
		id?: string | undefined
		class?: string | undefined
		action?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		title,
		noPadding = false,
		fullHeight = true,
		titlePadding = '',
		tooltip = '',
		documentationLink = undefined,
		id = undefined,
		class: clazz = undefined,
		action,
		children
	}: Props = $props()
</script>

<div
	class={classNames(
		clazz,
		'flex flex-col gap-2 items-start',
		noPadding ? '' : 'p-3',
		fullHeight ? 'h-full' : ''
	)}
	{id}
>
	<div class="flex justify-between flex-wrap items-center w-full gap-1">
		<div class="text-xs inline-flex items-center font-semibold text-primary {titlePadding} gap-1">
			<span class="truncate">
				{title}
			</span>
			{#if tooltip}
				<Tooltip {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</div>
		{@render action?.()}
	</div>
	{@render children?.()}
</div>
