<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { HorizontalAlignment, VerticalAlignment } from '../../types'
	import { tailwindHorizontalAlignment, tailwindVerticalAlignment } from '../../utils'

	interface Props {
		horizontalAlignment?: HorizontalAlignment | undefined
		verticalAlignment?: VerticalAlignment | undefined
		noWFull?: boolean
		hFull?: boolean
		class?: string
		style?: string
		render?: boolean
		children?: import('svelte').Snippet
	}

	let {
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		noWFull = false,
		hFull = false,
		class: c = '',
		style = '',
		render = true,
		children
	}: Props = $props()

	let classes = $derived(
		twMerge(
			'flex z-auto',
			noWFull ? '' : 'w-full',
			tailwindHorizontalAlignment(horizontalAlignment),
			tailwindVerticalAlignment(verticalAlignment),
			verticalAlignment || hFull ? 'h-full' : '',
			c
		)
	)
</script>

{#if render}
	<div class={classes} {style}>
		{@render children?.()}
	</div>
{/if}
