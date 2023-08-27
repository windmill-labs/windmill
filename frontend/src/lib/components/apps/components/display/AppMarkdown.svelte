<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { classNames } from '$lib/utils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'mardowncomponent'> | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['mardowncomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined

	$: css = concatCustomCss($app.css?.mardowncomponent, customCss)

	const proseMapping = {
		sm: 'prose-sm',
		Default: 'prose-base',
		lg: 'prose-lg',
		xl: 'prose-xl',
		'2xl': 'prose-2xl'
	}
</script>

{#each Object.keys(components['mardowncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<div
	on:pointerdown={(e) => {
		e?.preventDefault()
	}}
	class={classNames(
		'h-full w-full overflow-y-auto prose',
		resolvedConfig?.size ? proseMapping[resolvedConfig.size] : '',
		css?.container?.class,
		'wm-markdown'
	)}
>
	<RunnableWrapper
		{outputs}
		{render}
		autoRefresh
		{componentInput}
		{id}
		bind:initializing
		bind:result
	>
		{#if result}
			{#key result}
				<Markdown md={result} />
			{/key}
		{/if}
	</RunnableWrapper>
</div>
