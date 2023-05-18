<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { HorizontalAlignment, VerticalAlignment } from '../../types'
	import { tailwindHorizontalAlignment, tailwindVerticalAlignment } from '../../utils'

	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let noWFull = false
	export let hFull = false
	let c = ''
	export { c as class }
	export let style = ''
	export let render: boolean = true

	$: classes = twMerge(
		'flex z-auto',
		noWFull ? '' : 'w-full',
		tailwindHorizontalAlignment(horizontalAlignment),
		tailwindVerticalAlignment(verticalAlignment),
		verticalAlignment || hFull ? 'h-full' : '',
		c
	)
</script>

{#if render}
	<div class={classes} {style}>
		<slot />
	</div>
{/if}
