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

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'displaycomponent'> | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations

	const requireHtmlApproval = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)
	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')
	let result: any = undefined

	const resolvedConfig = initConfig(
		components['displaycomponent'].initialData.configuration,
		configuration
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

	let css = initCss($app.css?.displaycomponent, customCss)
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

<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	<div class="flex flex-col w-full h-full">
		<div
			class={twMerge(
				'w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm',
				css?.header?.class,
				'wm-rich-result-header'
			)}
			style={css?.header?.style}
		>
			{resolvedConfig?.title ?? 'Result'}
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
			<DisplayResult {result} {requireHtmlApproval} disableExpand={resolvedConfig?.hideDetails} disableDetails={resolvedConfig?.hideDetails}/>
		</div>
	</div>
</RunnableWrapper>
