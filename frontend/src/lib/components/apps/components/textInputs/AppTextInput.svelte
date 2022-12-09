<script lang="ts">
	import type { AppInput } from '../../inputType'
	import DebouncedInput from '../helpers/DebouncedInput.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let extraQueryParams: Record<string, any> = {}
	export const staticOutputs: string[] = ['result']

	let value: string
	let labelValue: string = 'Title'
	let runnableComponent: RunnableComponent

	// $: if(value || !value) {
	// 	if(componentInput) {
	// 		(componentInput as any).value = value
	// 	}
	// 	runnableComponent?.runComponent()
	// }
</script>

<InputValue input={configuration.label} bind:value={labelValue} />

<RunnableWrapper
	bind:runnableComponent
	bind:componentInput
	bind:result={value}
	{id}
	{extraQueryParams}
	autoRefresh={true}
>
	<!-- svelte-ignore a11y-label-has-associated-control -->
	<label>
		<div>
			{labelValue}
		</div>
		<DebouncedInput bind:value={value} debounceDelay={300} type="text" placeholder="Type..." />
	</label>
</RunnableWrapper>
