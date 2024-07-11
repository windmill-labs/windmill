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
	import { twMerge } from 'tailwind-merge'

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

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue

		if (value) {
			const date = parseISO(value)
			selectedDay = String(date.getDate())
			selectedMonth = [
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
			][date.getMonth()]
			selectedYear = String(date.getFullYear())
		}
	}
	let css = initCss($app.css?.dateinputcomponent, customCss)

	let darkMode: boolean = false
	let selectedDay: string | undefined = undefined
	let selectedMonth: string | undefined = undefined
	let selectedYear: string | undefined = undefined

	function setOutput() {
		let dateParts = [] as string[]

		if (resolvedConfig.enableYear && selectedYear) {
			dateParts.push(selectedYear)
		} else {
			dateParts.push('1900')
		}

		if (resolvedConfig.enableMonth && selectedMonth) {
			const monthIndex =
				[
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
				].indexOf(selectedMonth) + 1

			dateParts.push(String(monthIndex).padStart(2, '0'))
		} else {
			dateParts.push('01')
		}

		if (resolvedConfig.enableDay && selectedDay) {
			dateParts.push(String(selectedDay).padStart(2, '0'))
		} else {
			dateParts.push('01')
		}

		const dateString = dateParts.join('-')

		if (dateParts.length > 0) {
			const formattedDate = formatDate(dateString, resolvedConfig.outputFormat)

			outputs?.result.set(formattedDate)
		} else {
			outputs?.result.set(undefined)
		}
	}
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
	<div
		class={twMerge(
			'flex w-full gap-2',
			resolvedConfig?.orientation === 'horizontal' ? 'flex-row' : 'flex-col',
			css?.container?.class
		)}
		style={css?.container?.style}
	>
		{#if resolvedConfig?.enableDay}
			<Select
				portal={false}
				value={selectedDay}
				on:change={(e) => {
					selectedDay = e.detail.value
					setOutput()
				}}
				on:clear={() => {
					selectedDay = ''
					setOutput()
				}}
				items={Array.from({ length: 31 }, (_, i) => {
					return { label: String(i + 1), value: String(i + 1) }
				})}
				class={twMerge('text-clip grow min-w-0', css?.input?.class, 'wm-date-select')}
				containerStyles={(darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles) + css?.input?.style}
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				placeholder="Select a day"
			/>
		{/if}
		{#if resolvedConfig?.enableMonth}
			<Select
				portal={false}
				value={selectedMonth}
				on:change={(e) => {
					selectedMonth = e.detail.value
					setOutput()
				}}
				on:clear={() => {
					selectedMonth = ''
					setOutput()
				}}
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
				placeholder="Select a month"
				class={twMerge('text-clip grow min-w-0', css?.input?.class, 'wm-date-select')}
				containerStyles={(darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles) + css?.input?.style}
				clearable
			/>
		{/if}
		{#if resolvedConfig?.enableYear}
			<Select
				portal={false}
				value={selectedYear}
				on:change={(e) => {
					selectedYear = e.detail.value
					setOutput()
				}}
				on:clear={() => {
					selectedYear = ''
					setOutput()
				}}
				items={Array.from({ length: 201 }, (_, i) => `${1900 + i}`)}
				placeholder="Select a year"
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				class={twMerge('text-clip grow min-w-0', css?.input?.class, 'wm-date-select')}
				containerStyles={(darkMode
					? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
					: SELECT_INPUT_DEFAULT_STYLE.containerStyles) + css?.input?.style}
				clearable
			/>
		{/if}
	</div>
</AlignWrapper>
