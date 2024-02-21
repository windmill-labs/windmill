<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		VerticalAlignment,
		RichConfigurations
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

	const { app, worldStore, componentControl, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['datetimeinputcomponent'].initialData.configuration,
		configuration
	)

	let value: string | undefined = undefined

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined,
		validity: true as boolean | undefined
	})

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
			outputs?.result.set(formatDate(value, resolvedConfig.outputFormat))
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
		} else {
			outputs?.result.set(undefined)
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}

	let css = initCss($app.css?.datetimeinputcomponent, customCss)
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
		componentStyle={$app.css?.datetimeinputcomponent}
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
				minDate={resolvedConfig.minDateTime
					? formatDate(resolvedConfig.minDateTime, 'yyyy-MM-dd')
					: undefined}
				maxDate={resolvedConfig.maxDateTime
					? formatDate(resolvedConfig.maxDateTime, 'yyyy-MM-dd')
					: undefined}
				on:focus={() => ($selectedComponent = [id])}
			/>
		{/if}
	</div>
</AlignWrapper>
