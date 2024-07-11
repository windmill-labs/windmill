<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { parseISO, format as formatDateFns } from 'date-fns'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import Select from '../../svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'dateselectcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['dateselectcomponent'].initialData.configuration,
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

	$: !value && handleDefault(resolvedConfig.defaultValue)

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
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}
	let css = initCss($app.css?.dateinputcomponent, customCss)

	let darkMode: boolean = false
	let selectedDay: number | undefined = undefined
</script>

{#each Object.keys(components['dateselectcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.dateselectcomponent}
	/>
{/each}

<DarkModeObserver bind:darkMode />

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<div class="flex flex-row w-full gap-2">
		{#if resolvedConfig?.enableDay}
			<Select
				portal={false}
				value={selectedDay}
				on:change={(e) => {}}
				on:clear={() => {}}
				items={[
					1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
					26, 27, 28, 29, 30, 31
				]}
				class="text-clip grow min-w-0"
				placeholder="Select a day"
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
				clearable
			/>
		{/if}
		{#if resolvedConfig?.enableMonth}
			<Select
				portal={false}
				value={selectedDay}
				on:change={(e) => {}}
				on:clear={() => {}}
				items={[
					'January',
					'February',
					'March',
					'April',
					'May',
					'June',
					'July',
					'August',
					'September',
					'October',
					'November',
					'December'
				]}
				class="text-clip grow min-w-0"
				placeholder="Select a month"
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
				clearable
			/>
		{/if}
		{#if resolvedConfig?.enableYear}
			<Select
				portal={false}
				value={selectedDay}
				on:change={(e) => {}}
				on:clear={() => {}}
				items={[
					2021, 2022, 2023, 2024, 2025, 2026, 2027, 2028, 2029, 2030, 2031, 2032, 2033, 2034, 2035,
					2036, 2037, 2038, 2039, 2040
				]}
				class="text-clip grow min-w-0"
				placeholder="Select a year"
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
				clearable
			/>
		{/if}
	</div>
</AlignWrapper>
