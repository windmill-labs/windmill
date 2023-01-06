<script lang="ts">
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Popover from './Popover.svelte'
</script>

<Popover notClickable class="flex">
	<Icon
		class="text-gray-500 font-thin inline-block align-middle w-4"
		data={faInfoCircle}
		scale={0.8}
	/>
	<span slot="text"> <slot /> </span>
</Popover>
