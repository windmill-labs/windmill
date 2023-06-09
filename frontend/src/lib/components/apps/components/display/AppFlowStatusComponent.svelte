<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { concatCustomCss } from '../../utils'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'flowstatuscomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, workspace } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	initializing = false

	$: css = concatCustomCss($app.css?.flowstatuscomponent, customCss)

	let jobId: string | undefined
</script>

<RunnableWrapper
	on:started={(e) => {
		jobId = e.detail
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
			Flow Status
		</div>
		<div
			style={twMerge(
				$app.css?.['flowstatuscomponent']?.['container']?.style,
				customCss?.container?.style
			)}
			class={twMerge(
				'p-2 grow overflow-auto',
				$app.css?.['flowstatuscomponent']?.['container']?.class,
				customCss?.container?.class
			)}
		>
			{#if jobId}
				<FlowStatusViewer workspaceId={workspace} {jobId} />
			{:else}
				<span class="text-gray-600 text-xs">No flow</span>
			{/if}
		</div>
	</div>
</RunnableWrapper>
