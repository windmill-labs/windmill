<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import type {
		ConnectedAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	export let componentInput: StaticAppInput | ConnectedAppInput | UserAppInput | RowAppInput
	export let userInputEnabled: boolean = false
</script>

{#if componentInput.type === 'connected'}
	<ConnectedInputEditor bind:componentInput />
{:else if componentInput.type === 'row'}
	<RowInputEditor bind:componentInput />
{:else}
	{#if userInputEnabled}
		<Toggle
			class="-mt-2 -mb-1"
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
