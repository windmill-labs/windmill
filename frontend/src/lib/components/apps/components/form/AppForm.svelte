<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { faArrowRight, faRefresh } from '@fortawesome/free-solid-svg-icons'
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
	let ownClick: boolean = false

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
			if (ownClick && !value) {
				ownClick = false
			}
		}
	})

	$: loading = isLoading && ownClick
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
	forceSchemaDisplay={true}
>
	<AlignWrapper {horizontalAlignment}>
		<div class="flex flex-col gap-2 px-4 w-full">
			<div>
				{#if componentInput?.type != 'runnable' || Object.values(componentInput?.fields ?? {}).filter((x) => x.type == 'user').length == 0}
					<span class="text-gray-600 italic text-sm py-2"
						>Run forms are meant to be associated with a runnable with some user inputs. Pick a
						runnable and set some 'Runnable Inputs' to 'User Input'</span
					>
				{/if}
			</div>
			<div class="flex justify-end">
				<Button
					{loading}
					btnClasses="mt-1"
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
