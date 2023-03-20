<script lang="ts">
	import { CustomColorPicker } from '../../../common'

	export let value = ''
	const style = {
		bgColor: ''
	}
</script>

<!-- svelte-ignore a11y-label-has-associated-control -->
<label class="flex pb-2">
	<div class="text-xs font-medium pb-0.5"> Background color </div>
	<CustomColorPicker bind:hex={style.bgColor} />
</label>
