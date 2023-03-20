<script lang="ts">
	import { CustomColorPicker } from '../../../../common'

	export let value: string
	let isOpen = false
</script>

<div class="color-picker-input">
	<input
		type="text"
		readonly
		{value}
		on:focus|preventDefault|stopPropagation={() => (isOpen = true)}
	/>
	<CustomColorPicker bind:hex={value} bind:isOpen />
</div>
