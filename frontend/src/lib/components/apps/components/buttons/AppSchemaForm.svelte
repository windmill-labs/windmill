<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput, selectId } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import LightweightSchemaForm from '$lib/components/LightweightSchemaForm.svelte'
	import type { Schema } from '$lib/common'
	import { concatCustomCss } from '../../utils'
	import { twMerge } from 'tailwind-merge'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'schemaformcomponent'> | undefined = undefined

	const { worldStore, connectingInput, app, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		values: {}
	})

	let result: Schema | undefined = undefined
	let args: Record<string, unknown> = {}

	function handleArgsChange() {
		const newArgs: Record<string, unknown> = {}

		for (const key in args) {
			if (result?.properties[key]) {
				newArgs[key] = args[key]
			}
		}

		outputs.values.set(newArgs, true)
	}

	$: args && handleArgsChange()
	$: css = concatCustomCss($app.css?.schemaformcomponent, customCss)

	const resolvedConfig = initConfig(
		components['schemaformcomponent'].initialData.configuration,
		configuration
	)
</script>

{#each Object.keys(components['schemaformcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	{#if result && Object.keys(result?.properties ?? {}).length > 0}
		<div
			class={twMerge('p-2 overflow-auto h-full', css?.container?.class)}
			style={css?.container?.style}
			on:pointerdown|stopPropagation={(e) =>
				!$connectingInput.opened && selectId(e, id, selectedComponent, $app)}
		>
			<LightweightSchemaForm
				schema={result}
				bind:args
				displayType={Boolean(resolvedConfig.displayType)}
				largeGap={Boolean(resolvedConfig.largeGap)}
				{css}
			/>
		</div>
	{:else}
		<p class="m-2 italic">Empty form (no propertie)</p>
	{/if}
</RunnableWrapper>
