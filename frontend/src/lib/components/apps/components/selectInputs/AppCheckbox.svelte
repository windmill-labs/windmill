<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentInputsSpec } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let componentInputs: ComponentInputsSpec
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = ['result']

	let labelValue: string = 'Default label'
	let value: boolean = false

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<boolean>
	}
</script>

<InputValue input={componentInputs.label} bind:value={labelValue} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<Toggle
		bind:value
		options={{ right: labelValue }}
		on:change={(e) => {
			outputs.result.set(e.detail)
		}}
	/>
</AlignWrapper>
