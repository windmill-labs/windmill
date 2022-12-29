<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = ['result']

	let labelValue: string = 'Default label'
	let value: boolean = false

	// As the checkbox is a special case and has no input
	// we need to manually set the output

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<boolean>
	}
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<Toggle
		on:pointerdown={(e) => {
			e?.stopPropagation()
			window.dispatchEvent(new Event('pointerup'))
		}}
		bind:value
		options={{ right: labelValue }}
		on:change={(e) => {
			outputs.result.set(e.detail)
		}}
	/>
</AlignWrapper>
