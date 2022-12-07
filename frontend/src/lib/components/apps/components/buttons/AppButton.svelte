<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import type { RunnableComponent } from '..'
	import type { ComponentInput, ComponentParameter } from '../../inputType'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: ComponentInput | undefined
	export let configuration: Record<string, ComponentParameter>

	export let extraQueryParams: Record<string, any> = {}

	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
</script>

<InputValue input={configuration.label} bind:value={labelValue} />
<InputValue input={configuration.color} bind:value={color} />
<InputValue input={configuration.size} bind:value={size} />

<RunnableWrapper
	bind:runnableComponent
	{componentInput}
	{id}
	{extraQueryParams}
	autoRefresh={false}
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
		</Button>
	</AlignWrapper>
</RunnableWrapper>
