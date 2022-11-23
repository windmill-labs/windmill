<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import type { ComponentInputsSpec, InputsSpec } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: ComponentInputsSpec

	export const staticOutputs: string[] = ['loading', 'result']

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let runnableComponent: RunnableComponent
</script>

<InputValue input={componentInputs.label} bind:value={labelValue} />
<InputValue input={componentInputs.color} bind:value={color} />

<RunnableComponent
	bind:this={runnableComponent}
	bind:inputs
	{path}
	{runType}
	{inlineScriptName}
	{id}
	autoRefresh={false}
>
	<Button
		on:click={() => {
			runnableComponent?.runComponent()
		}}
		btnClasses="h-full"
		{color}
	>
		{labelValue}
	</Button>
</RunnableComponent>
