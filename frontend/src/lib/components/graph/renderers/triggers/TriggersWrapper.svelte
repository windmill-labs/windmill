<script lang="ts">
	import { Maximize2, Minimize2 } from 'lucide-svelte'

	import { NODE } from '../../util'
	import Popover from '$lib/components/Popover.svelte'

	import { createEventDispatcher, getContext } from 'svelte'

	import TriggersBadge from './TriggersBadge.svelte'
	import InsertModuleButton from '../../../flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { twMerge } from 'tailwind-merge'
	import MapItem from '../../../flows/map/MapItem.svelte'
	import { getStateColor } from '../../util'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import type { TriggerContext } from '$lib/components/triggers'
	import { saveSchedule } from '$lib/components/flows/scheduleUtils'
	import { workspaceStore } from '$lib/stores'

	type TriggerType = 'webhooks' | 'emails' | 'schedules' | 'routes' | 'websockets'

	export let path: string
	export let newItem: boolean
	export let isFlow: boolean
	export let selected: boolean
	export let darkMode: boolean
	export let data: {
		modules: FlowModule[]
		index: number
		eventHandlers: GraphEventHandlers
		disableAi: boolean
		flowIsSimplifiable: boolean
	}

	const dispatch = createEventDispatcher()

	const { viewSimplifiedTriggers, selectedTrigger, primarySchedule, triggersCount } =
		getContext<TriggerContext>('TriggerContext')

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let simplifiedTriggers = true
	let triggerScriptModule: FlowModule | undefined = undefined
	$: triggerScriptModule = data.modules.find((mod) => mod.isTrigger)

	$: data.eventHandlers.simplifyFlow(simplifiedTriggers)

	let triggersToDisplay: TriggerType[]
	$: triggersToDisplay =
		simplifiedTriggers && data.flowIsSimplifiable
			? ['schedules']
			: ['schedules', 'webhooks', 'routes', 'websockets', 'emails']

	$: triggerItems = [
		{
			id: 1,
			type: 'text',
			data: { text: 'Triggers' },
			display: !(simplifiedTriggers && data.flowIsSimplifiable)
		},
		{
			id: 2,
			type: 'triggersBadge',
			data: { triggersToDisplay },
			display: true
		},
		{
			id: 3,
			type: 'insertButton',
			data: {},
			display: !data.flowIsSimplifiable || simplifiedTriggers,
			grow: true
		}
	]

	$: visibleTriggerItems = triggerItems.filter((item) => item.display)

	function updateFlowSchema() {
		if (!$flowStore || !$flowStateStore || !triggerScriptModule) {
			return
		}

		if ($flowStateStore[triggerScriptModule?.id ?? '']?.schema && $flowStore.schema) {
			const flowProperties = $flowStore.schema.properties || {}
			const stepProperties =
				$flowStateStore[triggerScriptModule?.id ?? '']?.schema?.properties || {}

			const mergedProperties = Object.assign({}, flowProperties, stepProperties)
			$flowStore.schema.properties = mergedProperties

			const flowRequired = Array.isArray($flowStore.schema.required)
				? $flowStore.schema.required
				: []
			const stepRequired = Array.isArray(
				$flowStateStore[triggerScriptModule?.id ?? '']?.schema?.required
			)
				? $flowStateStore[triggerScriptModule?.id ?? '']?.schema?.required ?? []
				: []
			const mergedRequired = [...new Set([...flowRequired, ...stepRequired])]
			$flowStore.schema.required = mergedRequired
		}
	}

	function removeStepSchemaFromFlowSchema(schemaToRemove: any) {
		if (!$flowStore || !$flowStore.schema || !schemaToRemove) {
			return
		}

		const stepProperties = schemaToRemove.properties || {}
		const stepRequired = schemaToRemove.required || []

		for (const prop in stepProperties) {
			if ($flowStore.schema.properties && $flowStore.schema.properties[prop]) {
				delete $flowStore.schema.properties[prop]
			}
		}

		if (Array.isArray($flowStore.schema.required)) {
			$flowStore.schema.required = $flowStore.schema.required.filter(
				(field) => !stepRequired.includes(field)
			)
		}
	}

	const callUpdateFlowSchema = () => updateFlowSchema()

	let prevTriggerSchema: any = null
	$: {
		const triggerSchemaId = triggerScriptModule?.id
		const triggerSchema = triggerSchemaId && $flowStateStore?.[triggerSchemaId]?.schema

		if (triggerSchema !== prevTriggerSchema) {
			if (prevTriggerSchema) {
				removeStepSchemaFromFlowSchema(prevTriggerSchema)
			}
			if (triggerSchema) {
				callUpdateFlowSchema()
			}
			prevTriggerSchema = triggerSchema
		}
	}

	$: $viewSimplifiedTriggers = simplifiedTriggers && data.flowIsSimplifiable

	function addSchedule() {
		$primarySchedule = {
			summary: 'Trigger Schedule',
			args: {},
			cron: '0 0 /5 * * * *',
			timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			enabled: true
		}
		$triggersCount = {
			...($triggersCount ?? {}),
			schedule_count: ($triggersCount?.schedule_count ?? 0) + 1,
			primary_schedule: { schedule: $primarySchedule.cron }
		}
		saveSchedule(path, newItem, $workspaceStore ?? '', primarySchedule, isFlow)
	}
</script>

<div style={`width: ${NODE.width}px;`} class="center-center">
	<button
		class="w-full border rounded-sm bg-surface shadow-md center-center items-center max-w-full
			{selected ? 'outline outline-offset-0  outline-2  outline-slate-500 dark:outline-gray-400' : ''}"
		on:click={() => dispatch('select', 'triggers')}
	>
		<div class="flex flex-row w-min-0 gap-2 w-fit max-w-full px-1 py-1">
			{#each visibleTriggerItems as item (item.id)}
				<div class="grow {item.grow ? 'grow' : 'shrink-0'} min-w-0 center-center">
					{#if item.type === 'triggersBadge'}
						<div class="flex flex-row gap-2">
							<TriggersBadge
								triggersToDisplay={item.data.triggersToDisplay}
								showOnlyWithCount={false}
								{path}
								{newItem}
								{isFlow}
								{selected}
								on:select
							/>
						</div>
					{:else if item.type === 'text'}
						<div class="grow min-w-0 items-center text-2xs font-normal">
							{item.data.text}
						</div>
					{:else if item.type === 'insertButton'}
						{#if !triggerScriptModule}
							<InsertModuleButton
								disableAi={data.disableAi}
								on:new={(e) => {
									dispatch('new', e.detail)
									simplifiedTriggers = true
									addSchedule()
								}}
								on:pickScript={(e) => {
									dispatch('pickScript', e.detail)
									simplifiedTriggers = true
									addSchedule()
								}}
								on:select={() => {
									dispatch('select', 'triggers')
									$selectedTrigger = 'scheduledPoll'
								}}
								kind="trigger"
								index={data?.index ?? 0}
								modules={data?.modules ?? []}
								buttonClasses={twMerge(
									'bg-surface hover:bg-surface-hover rounded-md border text-xs',
									'w-6 h-6',
									'relative center-center',
									'flex-shrink-0'
								)}
							/>
						{:else if simplifiedTriggers && data.flowIsSimplifiable}
							<div
								class="text-2xs text-secondary min-w-0 font-normal text-center rounded-sm grow shadow-md w-full border"
							>
								<MapItem
									mod={triggerScriptModule}
									insertable={false}
									bgColor={getStateColor(undefined, darkMode, true, false)}
									modules={data.modules ?? []}
									moving={''}
									flowJobs={undefined}
									on:select
									isTrigger={true}
								/>
							</div>
						{/if}
					{/if}
				</div>
			{/each}

			{#if data.flowIsSimplifiable}
				<div class="absolute -top-5 right-0 text-2xs">
					<Popover notClickable placement="auto">
						<button
							class="absolute top-[12px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
	outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-150 hover:bg-nord-950 hover:text-white"
							on:click|preventDefault|stopPropagation={() =>
								(simplifiedTriggers = !simplifiedTriggers)}
						>
							{#if simplifiedTriggers}
								<Maximize2 size={12} strokeWidth={2} />
							{:else}
								<Minimize2 size={12} strokeWidth={2} />
							{/if}
						</button>
						<svelte:fragment slot="text"
							>{simplifiedTriggers
								? 'Expand schedule poll nodes'
								: 'Collapse schedule poll nodes'}</svelte:fragment
						>
					</Popover>
				</div>
			{/if}
		</div>
	</button>
</div>
