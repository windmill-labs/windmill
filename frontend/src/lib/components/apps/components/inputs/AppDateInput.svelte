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

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['dateinputcomponent'].initialData.configuration,
		configuration
	)

	let value: string | undefined = undefined

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	$: handleDefault(resolvedConfig.defaultValue)

	function formatDate(dateString: string, formatString: string = 'dd.MM.yyyy') {
		const type = resolvedConfig?.type as 'date' | 'datetime-local' | 'time'
		if (formatString === '') {
			if (type === 'time') {
				formatString = 'HH:mm'
			} else if (type === 'date') {
				formatString = 'dd.MM.yyyy'
			} else {
				formatString = 'dd.MM.yyyy HH:mm'
			}
		}

		try {
			const isoDate = parseISO(type === 'time' ? `1970-01-01T${dateString}` : dateString)
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
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}
	let css = initCss($app.css?.dateinputcomponent, customCss)
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
		componentStyle={$app.css?.dateinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	{#if inputType === 'date'}
		{#if resolvedConfig.type === 'date'}
			<input
				on:focus={() => ($selectedComponent = [id])}
				on:pointerdown|stopPropagation
				bind:value
				type="date"
				min={resolvedConfig.minDate}
				max={resolvedConfig.maxDate}
				placeholder="Type..."
				class={twMerge(css?.input?.class, 'wm-date-input')}
				style={css?.input?.style ?? ''}
			/>
		{:else if resolvedConfig.type === 'datetime-local'}
			<input
				on:focus={() => ($selectedComponent = [id])}
				on:pointerdown|stopPropagation
				bind:value
				type="datetime-local"
				min={resolvedConfig.minDate}
				max={resolvedConfig.maxDate}
				placeholder="Type..."
				class={twMerge(css?.input?.class, 'wm-date-input')}
				style={css?.input?.style ?? ''}
			/>
		{:else if resolvedConfig.type === 'time'}
			<input
				on:focus={() => ($selectedComponent = [id])}
				on:pointerdown|stopPropagation
				bind:value
				type="time"
				min={resolvedConfig.minDate}
				max={resolvedConfig.maxDate}
				placeholder="Type..."
				class={twMerge(css?.input?.class, 'wm-date-input')}
				style={css?.input?.style ?? ''}
			/>
		{/if}
	{/if}
</AlignWrapper>
