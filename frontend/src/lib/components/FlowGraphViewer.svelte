<script lang="ts">
	import type { FlowModule, FlowValue, TriggersCount } from '$lib/gen'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'

	import { createEventDispatcher, hasContext, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'

	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { dfs } from './flows/dfs'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		flow: {
			summary: string
			description?: string
			value: FlowValue
			schema?: any
			path?: string
		}
		overflowAuto?: boolean
		noSide?: boolean
		download?: boolean
		noGraph?: boolean
		triggerNode?: boolean
		stepDetail?: FlowModule | string | undefined
		workspace?: string | undefined
		minHeight?: number
		noBorder?: boolean
		hideDefaultInputs?: boolean
		provideTriggerContext?: boolean
		fillAvailableHeight?: boolean
	}

	let {
		flow,
		overflowAuto = false,
		noSide = false,
		download = false,
		noGraph = false,
		triggerNode = false,
		stepDetail = $bindable(undefined),
		workspace = $workspaceStore,
		minHeight = 400,
		noBorder = false,
		hideDefaultInputs = false,
		provideTriggerContext = false,
		fillAvailableHeight = false
	}: Props = $props()

	let availableHeight = $state(0)

	if (provideTriggerContext && !hasContext('TriggerContext')) {
		const triggersCount = writable<TriggersCount | undefined>(undefined)
		setContext<TriggerContext>('TriggerContext', {
			triggersCount,
			simplifiedPoll: writable(false),
			showCaptureHint: writable(undefined),
			triggersState: new Triggers()
		})
	}

	const dispatch = createEventDispatcher()
</script>

<div bind:clientHeight={availableHeight} class="grid grid-cols-3 w-full h-full min-h-0">
	{#if !noGraph}
		<div
			class="{noSide || (hideDefaultInputs && stepDetail == undefined)
				? 'col-span-3'
				: 'sm:col-span-2 col-span-3'} w-full h-full min-h-0 max-h-full"
			class:overflow-auto={overflowAuto}
			class:border={!noBorder}
		>
			<FlowGraphV2
				{triggerNode}
				earlyStop={flow.value.skip_expr !== undefined}
				cache={flow.value.cache_ttl !== undefined}
				path={flow?.path}
				{download}
				minHeight={fillAvailableHeight ? Math.max(minHeight, availableHeight) : minHeight}
				{workspace}
				modules={flow?.value?.modules}
				failureModule={flow?.value?.failure_module}
				preprocessorModule={flow?.value?.preprocessor_module}
				notes={flow?.value?.notes}
				groups={flow?.value?.groups}
				onSelect={(nodeId) => {
					if (nodeId === 'Trigger') {
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
	{#if !noSide && !(hideDefaultInputs && stepDetail == undefined)}
		<div
			class={twMerge(
				fillAvailableHeight
					? 'relative w-full h-full min-h-0 border-r border-b border-t p-2 pt-0 overflow-auto hidden sm:flex flex-col gap-4'
					: 'relative w-full h-full min-h-[150px] max-h-[90vh] border-r border-b border-t p-2 pt-0 overflow-auto hidden sm:flex flex-col gap-4',
				noGraph ? 'border-0 w-max' : ''
			)}
		>
			<FlowGraphViewerStep schema={flow.schema} {stepDetail} {hideDefaultInputs} />
		</div>
	{/if}
</div>
