<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { HorizontalAlignment, VerticalAlignment } from '../../types'

	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let noWFull = false

	function tailwindHorizontalAlignment(alignment?: HorizontalAlignment) {
		if(!alignment) return '';
		const classes: Record<HorizontalAlignment, string> = {
			left: 'justify-start',
			center: 'justify-center',
			right: 'justify-end',
		}
		return classes[alignment]
	}

	function tailwindVerticalAlignment(alignment?: VerticalAlignment) {
		if(!alignment) return '';
		const classes: Record<VerticalAlignment, string> = {
			top: 'items-start',
			center: 'items-center',
			bottom: 'items-end',
		}
		return classes[alignment]
	}

	$: classes = classNames(
		'flex z-auto',
		noWFull ? '' : 'w-full',
		tailwindHorizontalAlignment(horizontalAlignment),
		tailwindVerticalAlignment(verticalAlignment),
		verticalAlignment ? 'h-full' : '',
		$$props.class || ''
	)
</script>

<div class={classes}>
	<slot />
</div>
