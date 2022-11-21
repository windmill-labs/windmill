<script lang="ts">
	import type { StaticInput } from '../../types'
	import Toggle from '$lib/components/Toggle.svelte'

	export let input: StaticInput
	export let canHide: boolean
</script>

{#if canHide}
	<Toggle bind:checked={input.visible} options={{ right: 'Visible' }} />
{/if}

{#if input.fieldType === 'number'}
	<input type="number" bind:value={input.value} />
{:else if input.fieldType === 'textarea'}
	<textarea bind:value={input.value} />
{:else if input.fieldType === 'boolean'}
	<Toggle bind:checked={input.value} />
{:else}
	<input bind:value={input.value} />
{/if}
