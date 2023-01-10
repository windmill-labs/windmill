<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { getContext } from 'svelte'
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

	let isLoading: boolean = false

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

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />

<RunnableWrapper
	defaultUserInput
	noMinH
	bind:runnableComponent
	bind:componentInput
	{id}
	{extraQueryParams}
	autoRefresh={false}
	forceSchemaDisplay={true}
>
	<AlignWrapper {horizontalAlignment}>
		<div class="flex flex-col gap-2 px-4 w-full ">
			<div>
				{#if componentInput?.type != 'runnable' || Object.values(componentInput?.fields ?? {}).filter((x) => x.type == 'user').length == 0}
					<span class="text-gray-600 italic text-sm py-2">
						Run forms are associated with a runnable that has user inputs. Once a runnable is
						chosen, set some 'Runnable Inputs' to 'User Input'
					</span>
				{/if}
			</div>
			<div class="flex justify-end">
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
			</div>
		</div>
	</AlignWrapper>
</RunnableWrapper>
