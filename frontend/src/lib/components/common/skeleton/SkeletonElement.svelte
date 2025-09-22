<script lang="ts">
	import { HEIGHT_UNIT, type ISkeletonElement } from './model'

	interface Props {
		element: ISkeletonElement
	}

	let { element }: Props = $props()
</script>

<div
	class="animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms]"
	style="height: {element.h * HEIGHT_UNIT}px; width: {element.w}%; min-width: {element.minW}px;"
></div>
