<script lang="ts">
	import type { RowAppInput } from '../../../inputType'

	export let componentInput: RowAppInput | undefined
</script>

{#if componentInput}
	<input type="text" placeholder="column name" bind:value={componentInput.column} />
{/if}
