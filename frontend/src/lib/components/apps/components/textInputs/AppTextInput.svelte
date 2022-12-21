<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let inputType = 'text'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement
	let labelValue: string = 'Title'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}

	function handleInput() {
		outputs?.result.set(input.value)
	}
</script>

<InputValue input={configuration.label} bind:value={labelValue} />

<AlignWrapper {verticalAlignment}>
	<input
		type={inputType}
		bind:this={input}
		on:input={handleInput}
		placeholder="Type..."
		class="h-full"
	/>
</AlignWrapper>
