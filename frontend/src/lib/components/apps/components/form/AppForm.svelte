<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { faUser } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
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

	export const staticOutputs: string[] = ['loading', 'result']

	const { runnableComponents, worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
	let goto: string | undefined = undefined

	let isLoading: boolean = false

	$: noInputs =
		componentInput?.type != 'runnable' || Object.keys(componentInput?.fields ?? {}).length == 0

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<Array<any>>
		loading: Output<boolean>
	}

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	$: outputs?.loading.subscribe({
		next: (value) => {
			isLoading = value
		}
	})
</script>

<InputValue {id} input={configuration.goto} bind:value={goto} />
<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />

<RunnableWrapper
	defaultUserInput
	bind:runnableComponent
	bind:componentInput
	{id}
	{goto}
	{extraQueryParams}
	autoRefresh={false}
	forceSchemaDisplay={true}
	runnableClass="!block"
>
	<AlignWrapper {horizontalAlignment}>
		<div class="flex flex-col gap-2 px-4 w-full">
			<div>
				{#if noInputs}
					<div class="text-gray-600 italic text-sm my-4">
						Run forms are associated with a runnable that has user inputs.
						<br />
						Once a script or flow is chosen, set some <strong>Runnable Inputs</strong> to
						<strong>
							User Input
							<Icon data={faUser} scale={1.3} class="rounded-sm bg-gray-200 p-1 ml-0.5" />
						</strong>
					</div>
				{/if}
			</div>
			<div class="flex justify-end">
				{#if !noInputs}
					<Button
						loading={isLoading}
						btnClasses="my-1"
						on:pointerdown={(e) => {
							e?.stopPropagation()
							window.dispatchEvent(new Event('pointerup'))
						}}
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
				{/if}
			</div>
		</div>
	</AlignWrapper>
</RunnableWrapper>
