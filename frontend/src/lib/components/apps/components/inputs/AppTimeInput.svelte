<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'timeinputcomponent'> | undefined
		render: boolean
		onChange?: string[] | undefined
	}

	let {
		id,
		configuration,
		verticalAlignment = undefined,
		customCss = undefined,
		render,
		onChange = undefined
	}: Props = $props()

	const { app, worldStore, selectedComponent, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['timeinputcomponent'].initialData.configuration, configuration)
	)

	let value: string | undefined = $state(undefined)

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined,
		validity: true as boolean
	})

	function convertToMinutes(time: string) {
		const [hours, minutes] = time.split(':').map(Number)
		return hours * 60 + minutes
	}

	function fireOnChange() {
		if (onChange) {
			onChange.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}
	let css = $state(initCss($app.css?.timeinputcomponent, customCss))
	$effect.pre(() => {
		resolvedConfig.defaultValue
		!value && untrack(() => handleDefault(resolvedConfig.defaultValue))
	})
	$effect.pre(() => {
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
	})
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
		onfocus={() => ($selectedComponent = [id])}
		onpointerdown={stopPropagation(() => ($selectedComponent = [id]))}
		type="time"
		bind:value
		min={resolvedConfig.minTime}
		max={resolvedConfig.maxTime}
		placeholder="Type..."
		class={twMerge('windmillapp w-full py-1.5 text-sm px-2', css?.input?.class)}
		style={css?.input?.style ?? ''}
	/>
</AlignWrapper>
