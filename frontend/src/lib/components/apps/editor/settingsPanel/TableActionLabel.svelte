<script lang="ts">
	import InputValue from '../../components/helpers/InputValue.svelte'
	import type { AppInput } from '../../inputType'

	export let componentInput: AppInput | undefined
	let label: string = ''
</script>

{#if componentInput !== undefined}
	<InputValue input={componentInput} bind:value={label} />
{/if}

{label}
