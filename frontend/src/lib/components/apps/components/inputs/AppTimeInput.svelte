<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'timeinputcomponent'> | undefined = undefined
	export let render: boolean
	export let onChange: string[] | undefined = undefined

	const { app, worldStore, selectedComponent, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['timeinputcomponent'].initialData.configuration,
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
		validity: true as boolean
	})

	$: !value && handleDefault(resolvedConfig.defaultValue)

	function convertToMinutes(time: string) {
		const [hours, minutes] = time.split(':').map(Number)
		return hours * 60 + minutes
	}

	$: {
		if (value) {
			if (!resolvedConfig['24hFormat']) {
				let time = value.split(':')
				let hours = parseInt(time[0])
				let minutes = time[1]
				let ampm = hours >= 12 ? 'pm' : 'am'
				hours = hours % 12
				hours = hours ? hours : 12

				outputs?.result.set(hours + ':' + minutes + ' ' + ampm)
			} else {
				outputs?.result.set(value)
			}

			let currentValueInMinutes = convertToMinutes(value)
			let isValid = true
			if (resolvedConfig.minTime) {
				const minTimeInMinutes = convertToMinutes(resolvedConfig.minTime)
				if (currentValueInMinutes < minTimeInMinutes) {
					isValid = false
				}
			}

			if (resolvedConfig.maxTime) {
				const maxTimeInMinutes = convertToMinutes(resolvedConfig.maxTime)
				if (currentValueInMinutes > maxTimeInMinutes) {
					isValid = false
				}
			}

			// At the end, set the validity
			outputs?.validity.set(isValid)
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
	let css = initCss($app.css?.timeinputcomponent, customCss)
</script>

{#each Object.keys(components['timeinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.timeinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		on:focus={() => ($selectedComponent = [id])}
		on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}
		type="time"
		bind:value
		min={resolvedConfig.minTime}
		max={resolvedConfig.maxTime}
		placeholder="Type..."
		class={twMerge('windmillapp w-full py-1.5 text-sm px-2', css?.input?.class)}
		style={css?.input?.style ?? ''}
	/>
</AlignWrapper>
