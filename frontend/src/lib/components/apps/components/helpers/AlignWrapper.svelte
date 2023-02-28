<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { HorizontalAlignment, VerticalAlignment } from '../../types'

	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let noWFull = false
	let c = ''
	export { c as class }
	export let style = ''

	function tailwindHorizontalAlignment(alignment?: HorizontalAlignment) {
		if (!alignment) return ''
		const classes: Record<HorizontalAlignment, string> = {
			left: 'justify-start',
			center: 'justify-center',
			right: 'justify-end'
		}
		return classes[alignment]
	}

	function tailwindVerticalAlignment(alignment?: VerticalAlignment) {
		if (!alignment) return ''
		const classes: Record<VerticalAlignment, string> = {
			top: 'items-start',
			center: 'items-center',
			bottom: 'items-end'
		}
		return classes[alignment]
	}

	$: classes = twMerge(
		'flex z-auto',
		noWFull ? '' : 'w-full',
		tailwindHorizontalAlignment(horizontalAlignment),
		tailwindVerticalAlignment(verticalAlignment),
		verticalAlignment ? 'h-full' : '',
		c
	)
</script>

<div class={classes} {style}>
	<slot />
</div>
