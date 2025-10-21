<script lang="ts">
	import { classNames } from '$lib/utils'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
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
		size?: 'lg' | 'md' | 'sm' | 'xs'
		collapsible?: boolean
		initiallyCollapsed?: boolean
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
		children,
		size = 'xs',
		collapsible = false,
		initiallyCollapsed = false
	}: Props = $props()

	function textSize() {
		switch (size) {
			case 'lg':
				return 'text-lg'
			case 'md':
				return 'text-md'
			case 'sm':
				return 'text-sm'
			case 'xs':
			default:
				return 'text-xs'
		}
	}

	let collapsed = $state(initiallyCollapsed)
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
		<div
			class="{textSize()} inline-flex items-center font-semibold text-primary {titlePadding} gap-1"
		>
			<span class="truncate">
				{title}
			</span>
			{#if tooltip}
				<Tooltip {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</div>
		{#if collapsible}
			<button class="flex items-center gap-1" onclick={() => (collapsed = !collapsed)}>
				{#if collapsed}
					<ChevronRight size={16} />
				{:else}
					<ChevronDown size={16} />
				{/if}
			</button>
		{/if}
		{@render action?.()}
	</div>
	{#if !collapsed}
		{@render children?.()}
	{/if}
</div>
