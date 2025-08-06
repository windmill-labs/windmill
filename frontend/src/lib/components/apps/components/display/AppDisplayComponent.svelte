<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import {
		IS_APP_PUBLIC_CONTEXT_KEY,
		type AppViewerContext,
		type ComponentCustomCSS,
		type RichConfigurations
	} from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'displaycomponent'> | undefined
		render: boolean
		configuration: RichConfigurations
	}

	let result_stream: string | undefined = $state(undefined)
	let {
		id,
		componentInput,
		initializing = $bindable(undefined),
		customCss = undefined,
		render,
		configuration
	}: Props = $props()

	const requireHtmlApproval = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)
	const { app, worldStore, componentControl, workspace, appPath } =
		getContext<AppViewerContext>('AppViewerContext')

	let result: any = $state(undefined)

	const resolvedConfig = $state(
		initConfig(components['displaycomponent'].initialData.configuration, configuration)
	)

	$componentControl[id] = {
		setValue(value: string) {
			result = value
		}
	}

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let css = $state(initCss($app.css?.displaycomponent, customCss))
	let loading = $state(false)
</script>

{#each Object.keys(components['displaycomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.displaycomponent}
	/>
{/each}

<RunnableWrapper
	{outputs}
	{render}
	{componentInput}
	{id}
	bind:initializing
	bind:result
	bind:loading
>
	<div class="flex flex-col w-full h-full component-wrapper">
		<div
			class={twMerge(
				'w-full border-b p-2 text-xs font-semibold text-primary bg-surface-secondary',
				css?.header?.class,
				'wm-rich-result-header'
			)}
			style={css?.header?.style}
		>
			{resolvedConfig?.title ? resolvedConfig?.title : 'Result'}
		</div>
		<div
			style={twMerge(
				$app.css?.['displaycomponent']?.['container']?.style,
				customCss?.container?.style,
				'wm-rich-result-container'
			)}
			class={twMerge(
				'p-2 grow overflow-auto',
				$app.css?.['displaycomponent']?.['container']?.class,
				customCss?.container?.class
			)}
		>
			<DisplayResult
				{loading}
				workspaceId={workspace}
				{result}
				{result_stream}
				{requireHtmlApproval}
				disableExpand={resolvedConfig?.hideDetails}
				appPath={$userStore ? undefined : $appPath}
				forceJson={resolvedConfig?.forceJson}
			/>
		</div>
	</div>
</RunnableWrapper>
