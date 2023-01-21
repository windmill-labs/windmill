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
	import Portal from 'svelte-portal'
	import Modal from '$lib/components/common/modal/Modal.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	export const staticOutputs: string[] = ['loading', 'result']

	const { runnableComponents, worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
	let disabled: boolean | undefined = undefined

	let isLoading: boolean = false
	let ownClick: boolean = false

	let errors: Record<string, string> = {}
	$: errorsMessage = Object.values(errors)
		.filter((x) => x != '')
		.join('\n')

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
			if (ownClick && !value) {
				ownClick = false
			}
		}
	})

	$: loading = isLoading && ownClick

	let open: boolean = false
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue
	row={extraQueryParams['row']}
	{id}
	input={configuration.disabled}
	bind:value={disabled}
	bind:error={errors.disabled}
/>

<Portal>
	<Modal
		{open}
		title={labelValue}
		on:canceled={() => {
			open = false
		}}
		on:confirmed={() => {
			open = false
		}}
	>
		<RunnableWrapper
			defaultUserInput
			noMinH
			bind:runnableComponent
			bind:componentInput
			{id}
			{extraQueryParams}
			autoRefresh={false}
			forceSchemaDisplay={true}
			runnableClass="!block"
		>
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
					<Button
						{loading}
						btnClasses="my-1"
						on:pointerdown={(e) => {
							e?.stopPropagation()
							window.dispatchEvent(new Event('pointerup'))
						}}
						on:click={async () => {
							await runnableComponent?.runComponent()

							if (recomputeIds) {
								recomputeIds.forEach((id) => {
									$runnableComponents[id]?.()
								})
							}

							open = false
						}}
						size="xs"
						color="dark"
					>
						Submit
					</Button>
				</div>
			</div>
		</RunnableWrapper>
	</Modal>
</Portal>

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	{#if errorsMessage}
		<div class="text-red-500 text-xs">{errorsMessage}</div>
	{/if}
	<Button
		{disabled}
		{size}
		{color}
		on:click={(e) => {
			open = true
		}}
	>
		{labelValue}
	</Button>
</AlignWrapper>
