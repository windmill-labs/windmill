<script lang="ts">
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'

	export let light = false
	export let scale = 0.8
	export let wrapperClass = ''
	export let placement: PopoverPlacement | undefined = undefined
</script>

<Popover notClickable {placement} class={wrapperClass}>
	<Icon
		class="{light
			? 'text-gray-300'
			: ' text-gray-500'} font-thin inline-block align-middle w-4 {$$props.class}"
		data={faInfoCircle}
		{scale}
	/>
	<span slot="text"> <slot /> </span>
</Popover>
