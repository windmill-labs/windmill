<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import {
		IS_APP_PUBLIC_CONTEXT_KEY,
		type AppViewerContext,
		type ComponentCustomCSS,
		type RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { userStore } from '$lib/stores'

	interface Props {
		id: string
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'jobiddisplaycomponent'> | undefined
		configuration: RichConfigurations
		render: boolean
	}

	let {
		id,
		initializing = $bindable(undefined),
		customCss = undefined,
		configuration,
		render
	}: Props = $props()

	const { app, worldStore, workspace, appPath } = getContext<AppViewerContext>('AppViewerContext')
	const requireHtmlApproval = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)

	let resolvedConfig = $state(
		initConfig(components['jobiddisplaycomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	initializing = false

	let css = $state(initCss($app.css?.jobiddisplaycomponent, customCss))

	let testJobLoader: TestJobLoader | undefined = $state(undefined)
	let testIsLoading: boolean = $state(false)
	let testJob: Job | undefined = $state(undefined)

	$effect(() => {
		if (resolvedConfig.jobId) {
			untrack(() => {
				outputs.loading.set(true)
				const jobId = resolvedConfig?.['jobId']
				if (jobId) {
					testJobLoader?.watchJob(jobId)
				}
			})
		}
	})

	let result: any = $state(undefined)
</script>

{#each Object.keys(components['jobiddisplaycomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.jobiddisplaycomponent}
	/>
{/each}

<TestJobLoader
	noCode={true}
	workspaceOverride={workspace}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
	on:done={(e) => {
		outputs.loading.set(false)
		outputs.jobId.set(e.detail.id)
		outputs.result.set(e.detail.result)
		result = e.detail.result
	}}
/>

<InitializeComponent {id} />
{#if render}
	<div class="flex flex-col w-full h-full component-wrapper">
		<div
			class={twMerge(
				'w-full border-b p-2 text-xs font-semibold text-primary bg-surface-secondary',
				css?.header?.class
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
				workspaceId={workspace}
				{result}
				{requireHtmlApproval}
				disableExpand={resolvedConfig?.hideDetails}
				appPath={$userStore ? undefined : $appPath}
				forceJson={resolvedConfig?.forceJson}
			/>
		</div>
	</div>
{/if}
