<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		VerticalAlignment,
		RichConfigurations,
		ListContext,
		ListInputs
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import DateTimeInput from '$lib/components/DateTimeInput.svelte'
	import { twMerge } from 'tailwind-merge'
	import { parseISO, format as formatDateFns } from 'date-fns'

	export let id: string
	export let configuration: RichConfigurations
	export let inputType: 'date'
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let customCss: ComponentCustomCSS<'datetimeinputcomponent'> | undefined = undefined
	export let render: boolean
	export let onChange: string[] | undefined = undefined
	const { app, worldStore, componentControl, selectedComponent, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = initConfig(
		components['datetimeinputcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined,
		validity: true as boolean | undefined
	})

	let initValue = outputs?.result.peak()
	let value: string | undefined =
		!iterContext && initValue != undefined ? initValue : resolvedConfig.defaultValue

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
			outputs?.result.set(value)
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let initialHandleDefault = true

	$: handleDefault(resolvedConfig.defaultValue)

	function formatDate(dateString: string, formatString: string = 'dd.MM.yyyy HH:mm') {
		if (formatString === '') {
			formatString = 'dd.MM.yyyy HH:mm'
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
			const r = formatDate(value, resolvedConfig.outputFormat)
			outputs?.result.set(r)
			const valueDate = new Date(value)

			if (resolvedConfig.minDateTime) {
				const minDate = new Date(resolvedConfig.minDateTime)
				if (
					minDate.getDay() === valueDate.getDay() &&
					minDate.getMonth() === valueDate.getMonth() &&
					minDate.getFullYear() === valueDate.getFullYear()
				) {
					outputs?.validity.set(minDate.getTime() < valueDate.getTime())
				}

				if (minDate.getTime() > valueDate.getTime()) {
					outputs?.validity.set(false)
				}
			}

			if (resolvedConfig.maxDateTime) {
				const maxDate = new Date(resolvedConfig.maxDateTime)

				if (
					maxDate.getDay() === valueDate.getDay() &&
					maxDate.getMonth() === valueDate.getMonth() &&
					maxDate.getFullYear() === valueDate.getFullYear()
				) {
					outputs?.validity.set(maxDate.getTime() > valueDate.getTime())
				}

				if (maxDate.getTime() < valueDate.getTime()) {
					outputs?.validity.set(false)
				}
			}
			if (iterContext && listInputs) {
				listInputs.set(id, value)
			}
		} else {
			outputs?.result.set(undefined)
			if (iterContext && listInputs) {
				listInputs.set(id, undefined)
			}
		}
		fireOnChange()
	}

	function fireOnChange() {
		if (onChange) {
			onChange.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		if (initialHandleDefault) {
			initialHandleDefault = false
			if (value != undefined) {
				return
			}
		}
		value = defaultValue
	}

	let css = initCss(app.val.css?.datetimeinputcomponent, customCss)
</script>

{#each Object.keys(components['datetimeinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.datetimeinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<div class={twMerge(css?.container?.class, 'w-full')} style={css?.container?.style}>
		{#if inputType === 'date'}
			<DateTimeInput
				bind:value
				useDropdown={resolvedConfig?.displayPresets}
				on:pointerdown={(e) => {
					e.stopPropagation()
					$selectedComponent = [id]
				}}
				inputClass={twMerge('windmillapp w-full py-1.5 px-2 text-sm', 'app-editor-input')}
				minDate={resolvedConfig.minDateTime
					? formatDate(resolvedConfig.minDateTime, 'yyyy-MM-dd')
					: undefined}
				maxDate={resolvedConfig.maxDateTime
					? formatDate(resolvedConfig.maxDateTime, 'yyyy-MM-dd')
					: undefined}
				on:focus={() => ($selectedComponent = [id])}
				disabled={resolvedConfig.disabled}
			/>
		{/if}
	</div>
</AlignWrapper>
