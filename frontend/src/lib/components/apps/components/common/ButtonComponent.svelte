<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentInputsSpec, InputsSpec } from '../../types'
	import ComponentInputValue from '../helpers/ComponentInputValue.svelte'
	import RunnableComponent from '../helpers/RunnableComponent.svelte'
	import AlignWrapper from './AlignWrapper.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: ComponentInputsSpec
	export let horizontalAlignement: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignement: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = ['loading', 'result']

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<any>
		loading: Output<boolean>
	}

	let result: any
	let loading: boolean = false

	$: result && outputs.result?.set(result)
	$: loading && outputs.loading?.set(loading)

	let labelValue: string = 'Default label'

	let tick = 0
</script>

<ComponentInputValue input={componentInputs.label} bind:value={labelValue} />

<RunnableComponent
	bind:inputs
	bind:result
	bind:loading
	{path}
	{runType}
	{inlineScriptName}
	shouldTick={tick}
>
	<AlignWrapper {horizontalAlignement} {verticalAlignement}>
		<Button
			on:click={() => {
				tick = tick + 1
			}}
		>
			{labelValue}
		</Button>
	</AlignWrapper>
</RunnableComponent>
