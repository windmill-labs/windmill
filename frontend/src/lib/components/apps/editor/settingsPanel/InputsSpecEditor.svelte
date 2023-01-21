<script lang="ts">
	import type {
		ConnectedAppInput,
		EvalAppInput,
		RowAppInput,
		StaticAppInput,
		UserAppInput
	} from '../../inputType'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import EvalInputEditor from './inputEditor/EvalInputEditor.svelte'
	import RowInputEditor from './inputEditor/RowInputEditor.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	export let id: string
	export let componentInput:
		| StaticAppInput
		| ConnectedAppInput
		| UserAppInput
		| RowAppInput
		| EvalAppInput
	export let hasRows: boolean = false
</script>

{#if componentInput.type === 'connected'}
	<ConnectedInputEditor bind:componentInput />
{:else if componentInput.type === 'row'}
	<RowInputEditor bind:componentInput />
{:else if componentInput.type === 'static'}
	<StaticInputEditor bind:componentInput />
{:else if componentInput.type === 'eval'}
	<EvalInputEditor {hasRows} {id} bind:componentInput />
{:else if componentInput.type === 'user'}
	<span class="text-2xs italic text-gray-6f00">Field's value is set by the user</span>
{/if}
