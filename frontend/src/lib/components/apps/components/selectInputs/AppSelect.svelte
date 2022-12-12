<script lang="ts">
	import { getContext } from 'svelte'
	import Select from 'svelte-select'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export const staticOutputs: string[] = ['loading', 'result']
	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const { runnableComponents } = getContext<AppEditorContext>('AppEditorContext')
	let value
	let labelValue: string = 'Default label'
	let runnableComponent: RunnableComponent

	$: items = (componentInput as any)?.value || []
</script>

<InputValue input={configuration.label} bind:value={labelValue} />

<RunnableWrapper
	bind:runnableComponent
	bind:componentInput
	{id}
	{extraQueryParams}
	autoRefresh={false}
>
	<AlignWrapper {horizontalAlignment} {verticalAlignment}>
		<!-- svelte-ignore a11y-label-has-associated-control -->
		<label class="block w-full">
			<div>
				{labelValue}
			</div>
			<Select
				on:change={() => {
					runnableComponent?.runComponent()

					if (recomputeIds) {
						recomputeIds.forEach((id) => {
							$runnableComponents[id]?.()
						})
					}
				}}
				bind:justValue={value}
				{items}
				class="w-full"
				placeholder="Select an item"
			/>
		</label>
	</AlignWrapper>
</RunnableWrapper>
