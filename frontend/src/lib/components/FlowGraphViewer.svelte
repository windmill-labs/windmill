<script lang="ts">
	import type { FlowModule, FlowValue } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { dfs } from './flows/dfs'
	import { workspaceStore } from '$lib/stores'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
		path?: string
	}

	export let overflowAuto = false
	export let noSide = false
	export let download = false
	export let noGraph = false
	export let triggerNode = false
	export let stepDetail: FlowModule | string | undefined = undefined
	export let workspace: string | undefined = $workspaceStore

	const dispatch = createEventDispatcher()
</script>

<div class="grid grid-cols-3 w-full h-full">
	{#if !noGraph}
		<div
			class="{noSide ? 'col-span-3' : 'sm:col-span-2 col-span-3'} w-full border max-h-full"
			class:overflow-auto={overflowAuto}
		>
			<FlowGraphV2
				{triggerNode}
				earlyStop={flow.value.skip_expr !== undefined}
				cache={flow.value.cache_ttl !== undefined}
				path={flow?.path}
				{download}
				minHeight={400}
				{workspace}
				modules={flow?.value?.modules}
				failureModule={flow?.value?.failure_module}
				preprocessorModule={flow?.value?.preprocessor_module}
				onSelect={(nodeId) => {
					if (nodeId === 'triggers') {
						dispatch('triggerDetail')
						return
					} else if (nodeId === 'failure') {
						stepDetail = flow?.value?.failure_module
					} else if (nodeId === 'preprocessor') {
						stepDetail = flow?.value?.preprocessor_module
					} else {
						stepDetail = dfs(flow?.value?.modules ?? [], (m) => m).find((m) => m?.id === nodeId)
					}
					stepDetail = stepDetail ?? nodeId
					dispatch('select', stepDetail)
				}}
			/>
		</div>
	{/if}
	{#if !noSide}
		<div
			class={twMerge(
				'relative w-full h-full min-h-[150px] max-h-[90vh] border-r border-b border-t p-2 pt-0 overflow-auto hidden sm:flex flex-col gap-4',
				noGraph ? 'border-0 w-max' : ''
			)}
		>
			<FlowGraphViewerStep schema={flow.schema} {stepDetail} />
		</div>
	{/if}
</div>
