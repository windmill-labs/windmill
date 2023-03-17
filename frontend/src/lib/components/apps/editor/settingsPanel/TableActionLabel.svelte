<script lang="ts">
	import InputValue from '../../components/helpers/InputValue.svelte'
	import type { RichConfiguration } from '../../types'

	export let componentInput: RichConfiguration | undefined
	let label: string = ''
</script>

{#if componentInput !== undefined}
	<InputValue input={componentInput} bind:value={label} />
{/if}

{label}
