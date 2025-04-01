<script lang="ts">
	import { createTooltip } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'

	export let floatingConfig: any | undefined = undefined
	export let placement: Placement = 'bottom'
	export let disablePopup = false

	const {
		elements: { trigger, content, arrow },
		states: { open },
		options: { positioning: positioningOption }
	} = createTooltip({
		openDelay: 0,
		closeDelay: 0,
		closeOnPointerDown: false,
		forceVisible: true,
		onOpenChange: ({ curr, next }) => {
			if (next && disablePopup) {
				return curr
			}
			return next
		}
	})

	$: $positioningOption = floatingConfig ?? {
		placement
	}

	$: console.log($open)
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->

<slot open={$open} content={$content} arrow={$arrow} trigger={$trigger} />
