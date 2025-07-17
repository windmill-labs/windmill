<script lang="ts">
	import type { RowAppInput } from '../../../inputType'

	interface Props {
		componentInput: RowAppInput | undefined
	}

	let { componentInput = $bindable() }: Props = $props()
</script>

{#if componentInput}
	<input type="text" placeholder="column name" bind:value={componentInput.column} />
{/if}
