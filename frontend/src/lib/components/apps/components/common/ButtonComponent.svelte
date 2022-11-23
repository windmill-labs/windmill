<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import type { ComponentInputsSpec, InputsSpec } from '../../types'
	import ComponentInputValue from '../helpers/ComponentInputValue.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: ComponentInputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let runnableComponent: RunnableComponent
</script>

<ComponentInputValue input={componentInputs.label} bind:value={labelValue} />
<ComponentInputValue input={componentInputs.color} bind:value={color} />

<RunnableComponent
	bind:this={runnableComponent}
	bind:inputs
	{path}
	{runType}
	{inlineScriptName}
	{id}
	autoRefresh={false}
>
	<AlignWrapper {horizontalAlignement} {verticalAlignement}>
		<Button
			on:click={() => {
				runnableComponent?.runComponent()
			}}
			btnClasses="h-full"
			{color}
		>
			{labelValue}
		</Button>
	</AlignWrapper>
</RunnableComponent>
