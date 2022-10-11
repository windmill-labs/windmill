<script lang="ts">
	import { skeleton } from '../../../directives'
	import { HEIGHT_UNIT, type ISkeletonElement } from './model'

	export let element: ISkeletonElement
</script>

<div
	use:skeleton={true}
	style="height: {element.h * HEIGHT_UNIT}px; width: {element.w}%; min-width: {element.minW}px;"
/>
