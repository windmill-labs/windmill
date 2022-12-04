<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import type { InputsSpec } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: InputsSpec
	export let extraQueryParams: Record<string, any> = {}

	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

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
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<Button
			on:click={() => {
				runnableComponent?.runComponent()
			}}
			{size}
			{color}
		>
			{labelValue}
		</Button></AlignWrapper
	>
</RunnableComponent>
