<script lang="ts">
	import { HEIGHT_UNIT, type ISkeletonElement } from './model'

	export let element: ISkeletonElement
</script>

<div
	class="animate-skeleton [animation-delay:1000ms]"
	style="height: {element.h * HEIGHT_UNIT}px; width: {element.w}%; min-width: {element.minW}px;"
/>
