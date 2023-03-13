<script lang="ts">
	import { Button, type ButtonType } from '$lib/components/common'
	import { faUser } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import Portal from 'svelte-portal'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'button' | 'popup'> | undefined = undefined
	export let render: boolean

	export const staticOutputs: string[] = ['loading', 'result']

	const { app, runnableComponents, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let labelValue: string = 'Default label'
	let color: ButtonType.Color
	let size: ButtonType.Size
	let runnableComponent: RunnableComponent
	let disabled: boolean | undefined = undefined

	let isLoading: boolean = false
	let ownClick: boolean = false

	let errors: Record<string, string> = {}
	let open: boolean = false

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

	$: css = concatCustomCss($app?.css?.formbuttoncomponent, customCss)
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.color} bind:value={color} />
<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue
	{id}
	input={configuration.disabled}
	bind:value={disabled}
	bind:error={errors.disabled}
/>

<Portal>
	<Modal
		{open}
		title={labelValue}
		class={css?.popup.class}
		style={css?.popup.style}
		on:canceled={() => {
			open = false
		}}
		on:confirmed={() => {
			open = false
		}}
	>
		<RunnableWrapper
			{recomputeIds}
			{render}
			bind:runnableComponent
			{componentInput}
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
		btnClasses={css?.button?.class ?? ''}
		style={css?.button?.style ?? ''}
		on:click={(e) => {
			open = true
		}}
	>
		{labelValue}
	</Button>
</AlignWrapper>
