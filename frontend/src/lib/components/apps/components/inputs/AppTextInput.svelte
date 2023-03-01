<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputDefaultValue from '../helpers/InputDefaultValue.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let inputType = 'text'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'input'> | undefined = undefined
	export let appCssKey: 'textinputcomponent' | 'passwordinputcomponent' = 'textinputcomponent'

	const { app, worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement

	let placeholder: string | undefined = undefined
	let defaultValue: string | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}

	function handleInput() {
		outputs?.result.set(input.value)
	}

	$: input && handleInput()

	$: css = concatCustomCss($app.css?.[appCssKey], customCss)
</script>

<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InputDefaultValue bind:input {defaultValue} />

<AlignWrapper {verticalAlignment}>
	<input
		class={twMerge('mx-0.5', css?.input?.class ?? '')}
		style={css?.input?.style ?? ''}
		on:focus={(e) => {
			e?.stopPropagation()
			window.dispatchEvent(new Event('pointerup'))
		}}
		type={inputType}
		bind:this={input}
		on:input={handleInput}
		{placeholder}
	/>
</AlignWrapper>
