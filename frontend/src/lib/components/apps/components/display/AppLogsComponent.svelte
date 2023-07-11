<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { concatCustomCss } from '../../utils'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import type { Job } from '$lib/gen'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'logcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	initializing = false

	$: css = concatCustomCss($app.css?.logcomponent, customCss)

	let testJobLoader: TestJobLoader | undefined = undefined
	let testIsLoading: boolean = false
	let testJob: Job | undefined = undefined
</script>

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job={testJob} />

<RunnableWrapper
	on:started={(e) => {
		testJobLoader?.watchJob(e.detail)
	}}
	{outputs}
	{render}
	{componentInput}
	{id}
>
	<div class="flex flex-col w-full h-full">
		<div
			class={twMerge(
				'w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm',
				css?.header?.class
			)}
			style={css?.header?.style}
		>
			Logs
		</div>
		<div
			style={twMerge($app.css?.['logcomponent']?.['container']?.style, customCss?.container?.style)}
			class={twMerge(
				'p-2 grow overflow-auto',
				$app.css?.['logcomponent']?.['container']?.class,
				customCss?.container?.class
			)}
		>
			<LogViewer
				jobId={testJob?.id}
				duration={testJob?.['duration_ms']}
				mem={testJob?.['mem_peak']}
				content={testJob?.logs}
				isLoading={testIsLoading}
			/>
		</div>
	</div>
</RunnableWrapper>
