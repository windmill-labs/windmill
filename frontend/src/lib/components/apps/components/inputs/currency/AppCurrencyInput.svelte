<script lang="ts">
	import { initOutput } from '$lib/components/apps/editor/appUtils'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppInput } from '../../../inputType'
	import type { Output } from '../../../rx'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../../types'
	import { concatCustomCss } from '../../../utils'
	import AlignWrapper from '../../helpers/AlignWrapper.svelte'
	import InputValue from '../../helpers/InputValue.svelte'
	import CurrencyInput from './CurrencyInput.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'currencycomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: null as number | null
	})

	let defaultValue: number | undefined = undefined

	let isNegativeAllowed: boolean | undefined = undefined
	let currency: string | undefined = undefined
	let locale: string | undefined = undefined
	let value: number | undefined = undefined

	function handleInput() {
		outputs?.result.set(value ?? null)
	}

	function handleDefault() {
		value = defaultValue
		handleInput()
	}

	$: value != undefined && handleInput()

	$: defaultValue != undefined && handleDefault()

	$: css = concatCustomCss($app.css?.currencycomponent, customCss)
</script>

<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InputValue {id} input={configuration.isNegativeAllowed} bind:value={isNegativeAllowed} />
<InputValue {id} input={configuration.currency} bind:value={currency} />
<InputValue {id} input={configuration.locale} bind:value={locale} />

<AlignWrapper {render} {verticalAlignment}>
	{#key isNegativeAllowed}
		{#key locale}
			{#key currency}
				<div class="w-full" on:pointerdown|stopPropagation={() => ($selectedComponent = id)}>
					<CurrencyInput
						inputClasses={{
							formatted: twMerge('px-2 w-full py-1.5 windmillapp', css?.input?.class),
							wrapper: 'w-full windmillapp',
							formattedZero: twMerge('text-black windmillapp ', css?.input?.class)
						}}
						style={css?.input?.style}
						bind:value
						{currency}
						{locale}
						{isNegativeAllowed}
					/>
				</div>
			{/key}
		{/key}
	{/key}
</AlignWrapper>
