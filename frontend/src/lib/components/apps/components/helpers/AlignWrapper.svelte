<script lang="ts">
	import { classNames } from '$lib/utils'

	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined

	function tailwindHorizontalAlignment(horizontalAlignment) {
		switch (horizontalAlignment) {
			case 'left':
				return 'justify-start'
			case 'center':
				return 'justify-center'
			case 'right':
				return 'justify-end'
		}
	}

	function tailwindVerticalAlignment(verticalAlignment) {
		switch (verticalAlignment) {
			case 'top':
				return 'items-start'
			case 'center':
				return 'items-center'
			case 'bottom':
				return 'items-end'
		}
	}

	$: classes = classNames(
		'flex w-full h-full',
		tailwindHorizontalAlignment(horizontalAlignment),
		tailwindVerticalAlignment(verticalAlignment)
	)
</script>

<div class={classes}>
	<slot />
</div>
