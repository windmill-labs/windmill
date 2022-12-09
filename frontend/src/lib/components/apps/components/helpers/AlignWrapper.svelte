<script lang="ts">
	import { classNames } from '$lib/utils'

	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined

	function tailwindHorizontalAlignment(horizontalAlignment) {
		switch (horizontalAlignment) {
			case 'left':
				return 'items-start'
			case 'center':
				return 'items-center'
			case 'right':
				return 'items-end'
		}
	}

	function tailwindVerticalAlignment(verticalAlignment) {
		switch (verticalAlignment) {
			case 'top':
				return 'justify-start'
			case 'center':
				return 'justify-center'
			case 'bottom':
				return 'justify-end'
		}
	}

	$: classes = classNames(
		'flex w-full h-full flex-col',
		tailwindHorizontalAlignment(horizontalAlignment),
		tailwindVerticalAlignment(verticalAlignment)
	)
</script>

<div class={classes}>
	<slot />
</div>
