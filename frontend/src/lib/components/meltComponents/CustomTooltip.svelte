<script lang="ts">
	import { createTooltip, melt } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'

	export let floatingConfig: any | undefined = undefined
	export let placement: Placement = 'bottom'

	const {
		elements: { trigger, content, arrow },
		states: { open },
		options: { positioning: positioningOption }
	} = createTooltip({
		openDelay: 0,
		closeDelay: 0,
		closeOnPointerDown: false,
		forceVisible: true
	})

	$: $positioningOption = floatingConfig ?? {
		placement
	}

	$: console.log($open)
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div use:melt={$trigger} {...$$restProps} on:mouseenter on:mouseleave>
	<slot open={$open} content={$content} arrow={$arrow} />
</div>
