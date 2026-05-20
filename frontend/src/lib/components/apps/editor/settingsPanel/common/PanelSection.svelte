<script module lang="ts">
	// Subtle uppercase variant for sidebar section titles (raw-app etc.).
	export const SUBTLE_PANEL_TITLE = 'font-normal text-secondary text-2xs uppercase tracking-wider'
</script>

<script lang="ts">
	import { untrack } from 'svelte'
	import { classNames } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { ChevronDown, ChevronRight } from 'lucide-svelte'
	import Tooltip from '../../../../Tooltip.svelte'

	interface Props {
		title: string
		noPadding?: boolean
		fullHeight?: boolean
		titlePadding?: string
		titleClass?: string
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
		titleClass = 'font-semibold',
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

	let collapsed = $state(untrack(() => initiallyCollapsed))
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
			class={twMerge(
				textSize(),
				'inline-flex items-center text-primary gap-1',
				titleClass,
				titlePadding
			)}
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
