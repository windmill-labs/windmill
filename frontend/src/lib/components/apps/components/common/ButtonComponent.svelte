<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
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

	let runnableComponent: RunnableComponent
</script>

<InputValue input={componentInputs.label} bind:value={labelValue} />

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
		btnClasses="h-full w-full"
	>
		{labelValue}
	</Button>
</RunnableComponent>
