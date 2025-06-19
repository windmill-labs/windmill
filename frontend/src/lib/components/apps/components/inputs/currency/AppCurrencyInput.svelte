<script lang="ts">
	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import { getContext, onDestroy } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../../types'
	import { initCss } from '../../../utils'
	import AlignWrapper from '../../helpers/AlignWrapper.svelte'
	import CurrencyInput from './CurrencyInput.svelte'
	import InitializeComponent from '../../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import { components } from '$lib/components/apps/editor/component'
	import ResolveStyle from '../../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'currencycomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const outputs = initOutput($worldStore, id, {
		result: null as number | null
	})

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let resolvedConfig = initConfig(
		components['currencycomponent'].initialData.configuration,
		configuration
	)

	let initValue = outputs?.result.peak()
	let value: number | undefined =
		!iterContext && initValue != undefined ? initValue : resolvedConfig.defaultValue

	$componentControl[id] = {
		setValue(nvalue: number) {
			value = nvalue
			outputs?.result.set(value ?? null)
		}
	}

	function handleInput() {
		outputs?.result.set(value ?? null)
		if (iterContext && listInputs) {
			listInputs.set(id, value ?? null)
		}
	}

	let initialHandleDefault = true

	$: handleDefault(resolvedConfig.defaultValue)

	function handleDefault(dflt: number | undefined) {
		if (initialHandleDefault) {
			initialHandleDefault = false
			if (value != undefined) {
				return
			}
		}
		value = dflt
		handleInput()
	}

	$: value != undefined && handleInput()

	let css = initCss(app.val.css?.currencycomponent, customCss)
</script>

{#each Object.keys(components['currencycomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.currencycomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	{#key resolvedConfig.isNegativeAllowed}
		{#key resolvedConfig.locale}
			{#key resolvedConfig.currency}
				<div class="w-full" on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}>
					<CurrencyInput
						inputClasses={{
							formatted: twMerge(
								'px-2 text-sm w-full py-1.5 windmillapp app-editor-input',
								css?.input?.class,
								'wm-currency-input'
							),
							wrapper: 'w-full windmillapp',
							formattedZero: twMerge('windmillapp ', css?.input?.class, 'wm-currency')
						}}
						style={css?.input?.style}
						bind:value
						currency={resolvedConfig.currency}
						locale={resolvedConfig.locale}
						isNegativeAllowed={resolvedConfig.isNegativeAllowed}
					/>
				</div>
			{/key}
		{/key}
	{/key}
</AlignWrapper>
