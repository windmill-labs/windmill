<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />

<RunnableWrapper
	bind:runnableComponent
	bind:componentInput
	{id}
	{extraQueryParams}
	autoRefresh={false}
>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<Button
			on:click={() => {
				runnableComponent?.runComponent()

				if (recomputeIds) {
					recomputeIds.forEach((id) => {
						$runnableComponents[id]?.()
					})
				}
			}}
			{size}
			{color}
		>
			{labelValue}
		</Button>
	</AlignWrapper>
</RunnableWrapper>
