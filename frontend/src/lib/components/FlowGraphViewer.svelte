<script lang="ts">
	import { FlowGraph } from './graph'

	import type { FlowModule, FlowValue } from '$lib/gen'

	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let overflowAuto = false
	export let noSide = false
	export let download = false
	export let noGraph = false

	export let stepDetail: FlowModule | string | undefined = undefined

	const dispatch = createEventDispatcher()
</script>

<div class="grid grid-cols-3 w-full h-full">
	{#if !noGraph}
		<div
			class="{noSide ? 'col-span-3' : 'sm:col-span-2 col-span-3'} w-full border max-h-full"
			class:overflow-auto={overflowAuto}
		>
			<FlowGraph
				{download}
				minHeight={400}
				modules={flow?.value?.modules}
				failureModule={flow?.value?.failure_module}
				on:select={(e) => {
					stepDetail = e.detail
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
			<FlowGraphViewerStep {flow} {stepDetail} />
		</div>
	{/if}
</div>
