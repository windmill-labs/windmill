<script lang="ts">
	import ColorInput from '../settingsPanel/inputEditor/ColorInput.svelte'

	export let value = ''
	const style = {
		'background-color': ''
	}
</script>

<div class="bg-gray-100 rounded-md p-2">
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label class="block pb-2">
		<div class="text-xs font-medium pb-0.5"> Background color </div>
		<ColorInput bind:value={style['background-color']} />
	</label>
</div>
