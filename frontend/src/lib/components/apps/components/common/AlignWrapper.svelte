<script lang="ts">
	import { classNames } from '$lib/utils'

	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined

	function tailwindHorizontalAlignement(
		horizontalAlignement: 'left' | 'center' | 'right' | undefined
	) {
		switch (horizontalAlignement) {
			case 'left':
				return 'justify-start'
			case 'center':
				return 'justify-center'
			case 'right':
				return 'justify-end'
		}
	}

	function tailwindVerticalAlignement(verticalAlignement: 'top' | 'center' | 'bottom' | undefined) {
		switch (verticalAlignement) {
			case 'top':
				return 'items-start'
			case 'center':
				return 'items-center'
			case 'bottom':
				return 'items-end'
		}
	}

	$: classes = classNames(
		'flex w-full h-full flex-col',
		tailwindHorizontalAlignement(horizontalAlignement),
		tailwindVerticalAlignement(verticalAlignement)
	)
</script>

<div class={classes}>
	<slot />
</div>
