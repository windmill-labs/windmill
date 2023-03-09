<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let inputType = 'text'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'input'> | undefined = undefined
	export let appCssKey: 'textinputcomponent' | 'passwordinputcomponent' | 'emailinputcomponent' =
		'textinputcomponent'
	export let render: boolean

	const { app, worldStore, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let placeholder: string | undefined = undefined
	let defaultValue: string | undefined = undefined
	let value: string | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	$: handleDefault(defaultValue)

	$: outputs?.result.set(value)

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}

	$: css = concatCustomCss($app.css?.[appCssKey], customCss)
</script>

<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />

<AlignWrapper {render} {verticalAlignment}>
	{#if inputType === 'password'}
		<input
			class={twMerge(
				'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2 mx-0.5',
				css?.input?.class ?? ''
			)}
			style={css?.input?.style ?? ''}
			on:pointerdown|stopPropagation
			on:focus={() => ($selectedComponent = id)}
			type="password"
			bind:value
			{placeholder}
		/>
	{:else if inputType === 'text'}
		<input
			class={twMerge(
				'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2 mx-0.5',
				css?.input?.class ?? ''
			)}
			style={css?.input?.style ?? ''}
			on:pointerdown|stopPropagation
			on:focus={() => ($selectedComponent = id)}
			type="text"
			bind:value
			{placeholder}
		/>
	{:else if inputType === 'email'}
		<input
			class={twMerge(
				'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2 mx-0.5',
				css?.input?.class ?? ''
			)}
			style={css?.input?.style ?? ''}
			on:pointerdown|stopPropagation
			on:focus={() => ($selectedComponent = id)}
			type="email"
			bind:value
			{placeholder}
		/>
	{/if}
</AlignWrapper>
