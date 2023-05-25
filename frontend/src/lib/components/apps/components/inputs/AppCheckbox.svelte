<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let recomputeIds: string[] | undefined = undefined
	export let customCss: ComponentCustomCSS<'checkboxcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let preclickAction: (() => Promise<void>) | undefined = undefined

	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

	const { app, worldStore, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['checkboxcomponent'].initialData.configuration,
		configuration
	)

	if (controls) {
		$componentControl[id] = controls
	}

	// As the checkbox is a special case and has no input
	// we need to manually set the output

	let outputs = initOutput($worldStore, id, {
		result: false
	})

	$: resolvedConfig.defaultValue != undefined && outputs?.result.set(resolvedConfig.defaultValue)

	$: css = concatCustomCss($app.css?.checkboxcomponent, customCss)
</script>

{#each Object.keys(components['checkboxcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />
<AlignWrapper {render} {horizontalAlignment} {verticalAlignment}>
	<Toggle
		size="sm"
		bind:checked={resolvedConfig.defaultValue}
		options={{ right: resolvedConfig.label }}
		textClass={css?.text?.class ?? ''}
		textStyle={css?.text?.style ?? ''}
		on:change={(e) => {
			preclickAction?.()
			outputs.result.set(e.detail)
			if (recomputeIds) {
				recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb())
			}
		}}
	/>
</AlignWrapper>
