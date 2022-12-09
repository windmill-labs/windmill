<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let value: string
	let labelValue: string = 'Title'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}
	$: (value || !value) && outputs?.result.set(value || '')
</script>

<InputValue input={configuration.label} bind:value={labelValue} />

<!-- svelte-ignore a11y-label-has-associated-control -->
<label>
	<div>
		{labelValue}
	</div>
	<input type="text" bind:value placeholder="Type..." />
</label>
