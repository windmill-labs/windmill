<script lang="ts">
	import type { ComponentInput } from '../../inputType'
	import ConnectedInputEditor from './ConnectedInputEditor.svelte'
	import StaticInputEditor from './StaticInputEditor.svelte'

	export let componentInput: ComponentInput
	export let canHide: boolean = false
</script>

{#if componentInput.type === 'static'}
	<StaticInputEditor bind:componentInput {canHide} />
{:else if componentInput.type === 'connected'}
	<ConnectedInputEditor bind:componentInput />
{/if}
