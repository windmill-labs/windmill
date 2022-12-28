<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type { ConnectedAppInput, StaticAppInput, UserAppInput } from '../../inputType'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	export let componentInput: StaticAppInput | ConnectedAppInput | UserAppInput
	export let userInputEnabled: boolean = false
</script>

{#if componentInput.type === 'connected'}
	<ConnectedInputEditor bind:componentInput />
{:else}
	{#if userInputEnabled}
		<Toggle
			checked={componentInput.type === 'user'}
			on:change={({ detail }) => {
				componentInput.type = detail ? 'user' : 'static'
			}}
			options={{ right: 'User input' }}
			size="xs"
		/>
	{/if}
	{#if componentInput.type === 'static'}
		<StaticInputEditor bind:componentInput />
	{/if}
{/if}
