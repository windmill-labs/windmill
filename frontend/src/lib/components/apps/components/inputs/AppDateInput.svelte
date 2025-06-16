<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { parseISO, format as formatDateFns } from 'date-fns'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let inputType: 'date'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'dateinputcomponent'> | undefined = undefined
	export let render: boolean
	export let onChange: string[] | undefined = undefined

	const { app, worldStore, selectedComponent, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['dateinputcomponent'].initialData.configuration,
		configuration
	)

	let value: string | undefined = undefined

	$componentControl[id] = {
		setValue(nvalue: string) {
			if (typeof nvalue === 'string') {
				value = nvalue?.split('T')?.[0]
			} else {
				console.error('Invalid value', nvalue)
				value = undefined
			}
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	$: handleDefault(resolvedConfig.defaultValue)

	function formatDate(dateString: string, formatString: string = 'dd.MM.yyyy') {
		if (formatString === '') {
			formatString = 'dd.MM.yyyy'
		}

		try {
			const isoDate = parseISO(dateString)
			return formatDateFns(isoDate, formatString)
		} catch (error) {
			return 'Error formatting date:' + error.message
		}
	}

	$: {
		if (value) {
			outputs?.result.set(formatDate(value, resolvedConfig.outputFormat))
		} else {
			outputs?.result.set(undefined)
		}
		fireOnChange()
	}

	function fireOnChange() {
		if (onChange) {
			onChange.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}
	let css = initCss(app.val.css?.dateinputcomponent, customCss)
</script>

{#each Object.keys(components['dateinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.dateinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	{#if inputType === 'date'}
		<input
			on:focus={() => ($selectedComponent = [id])}
			on:pointerdown|stopPropagation
			type="date"
			bind:value
			disabled={resolvedConfig.disabled}
			min={resolvedConfig.minDate}
			max={resolvedConfig.maxDate}
			placeholder="Type..."
			class={twMerge(
				'windmillapp w-full py-1.5 text-sm px-1 app-editor-input',
				css?.input?.class,
				'wm-date-input'
			)}
			style={css?.input?.style ?? ''}
		/>
	{/if}
</AlignWrapper>
