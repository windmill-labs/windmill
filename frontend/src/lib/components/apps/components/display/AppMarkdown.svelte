<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { Markdown, type Plugin } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import rehypeRaw from 'rehype-raw'
	import { rehypeGithubAlerts } from 'rehype-github-alerts'
	import { classNames } from '$lib/utils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'mardowncomponent'> | undefined = undefined
	export let render: boolean
	export let configuration: RichConfigurations

	const plugins: Plugin[] = [
		gfmPlugin(),
		{ rehypePlugin: [rehypeRaw] },
		{ rehypePlugin: [rehypeGithubAlerts] }
	]
	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['mardowncomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined

	let css = initCss($app.css?.mardowncomponent, customCss)

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

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.mardowncomponent}
	/>
{/each}

{#if render}
	<div
		on:pointerdown={(e) => {
			if ($mode != 'preview') {
				e?.preventDefault()
			}
		}}
		class={classNames(
			'h-full w-full overflow-y-auto prose max-w-full',
			resolvedConfig?.size ? proseMapping[resolvedConfig.size] : '',
			css?.container?.class,
			'dark:prose-invert',
			'wm-markdown'
		)}
		style={css?.container?.style}
	>
		<RunnableWrapper
			{outputs}
			render={true}
			autoRefresh
			{componentInput}
			{id}
			bind:initializing
			bind:result
			>{#if result}{#key result}<Markdown md={result} {plugins} />{/key}{/if}
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} render={false} autoRefresh {componentInput} {id} />
{/if}

<style global>
	.wm-markdown p:first-child {
		margin-top: 0;
	}
	.wm-markdown p:last-child {
		margin-bottom: 0;
	}

	.wm-markdown ol:first-child {
		margin-top: 0;
	}
	.wm-markdown ol:last-child {
		margin-bottom: 0;
	}
	.wm-markdown ul:first-child {
		margin-top: 0;
	}
	.wm-markdown ul:last-child {
		margin-bottom: 0;
	}
	.wm-markdown li:first-child {
		margin-top: 0;
	}
	.wm-markdown li:last-child {
		margin-bottom: 0;
	}
	.wm-markdown h1:first-child {
		margin-top: 0;
	}
	.wm-markdown h1:last-child {
		margin-bottom: 0;
	}
	.wm-markdown h2:first-child {
		margin-top: 0;
	}
	.wm-markdown h2:last-child {
		margin-bottom: 0;
	}
	.wm-markdown h3:first-child {
		margin-top: 0;
	}
	.wm-markdown h3:last-child {
		margin-bottom: 0;
	}
	.wm-markdown h4:first-child {
		margin-top: 0;
	}
	.wm-markdown h4:last-child {
		margin-bottom: 0;
	}
	.wm-markdown code:not([class~="not-prose"]):not(.not-prose *)::before,
	.wm-markdown code:not([class~="not-prose"]):not(.not-prose *)::after {
		content: none !important;
	}
	.wm-markdown.prose code::before,
	.wm-markdown.prose code::after {
		content: none !important;
	}
</style>
