<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import type { InputsSpec } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: InputsSpec
	export let extraQueryParams: Record<string, any> = {}

	export const staticOutputs: string[] = ['loading', 'result']

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
</script>

<InputValue input={componentInputs.label} bind:value={labelValue} />
<InputValue input={componentInputs.color} bind:value={color} />
<InputValue input={componentInputs.size} bind:value={size} />

<RunnableComponent
	bind:this={runnableComponent}
	bind:inputs
	{path}
	{runType}
	{inlineScriptName}
	{id}
	autoRefresh={false}
	{extraQueryParams}
>
	<Button
		on:click={() => {
			runnableComponent?.runComponent()
		}}
		btnClasses="w-full h-full"
		{size}
		{color}
	>
		{labelValue}
	</Button>
</RunnableComponent>
