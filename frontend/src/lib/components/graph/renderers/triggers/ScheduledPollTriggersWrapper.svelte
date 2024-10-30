<script lang="ts">
	import { Maximize2, Minimize2, X } from 'lucide-svelte'

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
	export let simplifiedTrigger: undefined | { module: FlowModule |}

	const dispatch = createEventDispatcher()

	const { viewSimplifiedTriggers, selectedTrigger, primarySchedule, triggersCount } =
		getContext<TriggerContext>('TriggerContext')


	let hover = false

	// type TriggerKind = 'webhook' | 'email' | 'schedule' | 'route' | 'websocket'

	// const allTriggers: TriggerKind[] = ['webhook', 'email', 'schedule', 'route', 'websocket']
	// const simplifiedTriggers: 

	// let simplifiedTriggers = true
	// let triggerScriptModule: FlowModule | undefined = undefined
	// $: triggerScriptModule = data.modules.find(
	// 	(mod) =>
	// 		(mod.value.type == 'rawscript' && mod.value.is_trigger) ||
	// 		(mod.value.type == 'script' && mod.value.is_trigger)
	// )

	// $: data.eventHandlers.simplifyFlow(simplifiedTriggers)

	// let triggersToDisplay: TriggerType[]

	// $: triggerItems = [
	// 	{
	// 		id: 1,
	// 		type: 'text',
	// 		data: { text: 'Triggers' },
	// 		display: !(simplifiedTriggers && data.flowIsSimplifiable)
	// 	},
	// 	{
	// 		id: 2,
	// 		type: 'triggersBadge',
	// 		data: { triggersToDisplay },
	// 		display: true
	// 	},
	// 	{
	// 		id: 3,
	// 		type: 'insertButton',
	// 		data: {},
	// 		display: !data.flowIsSimplifiable || simplifiedTriggers,
	// 		grow: true
	// 	}
	// ]

	// $: visibleTriggerItems = triggerItems.filter((item) => item.display)

	// function updateFlowSchema() {
	// 	if (!$flowStore || !$flowStateStore || !triggerScriptModule) {
	// 		return
	// 	}

	// 	if ($flowStateStore[triggerScriptModule?.id ?? '']?.schema && $flowStore.schema) {
	// 		const flowProperties = $flowStore.schema.properties || {}
	// 		const stepProperties =
	// 			$flowStateStore[triggerScriptModule?.id ?? '']?.schema?.properties || {}

	// 		const mergedProperties = Object.assign({}, flowProperties, stepProperties)
	// 		$flowStore.schema.properties = mergedProperties

	// 		const flowRequired = Array.isArray($flowStore.schema.required)
	// 			? $flowStore.schema.required
	// 			: []
	// 		const stepRequired = Array.isArray(
	// 			$flowStateStore[triggerScriptModule?.id ?? '']?.schema?.required
	// 		)
	// 			? $flowStateStore[triggerScriptModule?.id ?? '']?.schema?.required ?? []
	// 			: []
	// 		const mergedRequired = [...new Set([...flowRequired, ...stepRequired])]
	// 		$flowStore.schema.required = mergedRequired
	// 	}
	// }

	// function removeStepSchemaFromFlowSchema(schemaToRemove: any) {
	// 	if (!$flowStore || !$flowStore.schema || !schemaToRemove) {
	// 		return
	// 	}

	// 	const stepProperties = schemaToRemove.properties || {}
	// 	const stepRequired = schemaToRemove.required || []

	// 	for (const prop in stepProperties) {
	// 		if ($flowStore.schema.properties && $flowStore.schema.properties[prop]) {
	// 			delete $flowStore.schema.properties[prop]
	// 		}
	// 	}

	// 	if (Array.isArray($flowStore.schema.required)) {
	// 		$flowStore.schema.required = $flowStore.schema.required.filter(
	// 			(field) => !stepRequired.includes(field)
	// 		)
	// 	}
	// }

	// const callUpdateFlowSchema = () => updateFlowSchema()

	// let prevTriggerSchema: any = null
	// $: {
	// 	const triggerSchemaId = triggerScriptModule?.id
	// 	const triggerSchema = triggerSchemaId && $flowStateStore?.[triggerSchemaId]?.schema

	// 	if (triggerSchema !== prevTriggerSchema) {
	// 		if (prevTriggerSchema) {
	// 			removeStepSchemaFromFlowSchema(prevTriggerSchema)
	// 		}
	// 		if (triggerSchema) {
	// 			callUpdateFlowSchema()
	// 		}
	// 		prevTriggerSchema = triggerSchema
	// 	}
	// }

	// $: $viewSimplifiedTriggers = simplifiedTriggers && data.flowIsSimplifiable

	// function addSchedule() {
	// 	let overridePrimarySchedule = !!$primarySchedule
	// 	$primarySchedule = {
	// 		summary: 'Trigger Schedule',
	// 		args: {},
	// 		cron: '0 */5 * * * *',
	// 		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
	// 		enabled: true
	// 	}
	// 	$triggersCount = {
	// 		...($triggersCount ?? {}),
	// 		schedule_count: ($triggersCount?.schedule_count ?? 0) + (overridePrimarySchedule ? 0 : 1),
	// 		primary_schedule: { schedule: $primarySchedule.cron }
	// 	}
	// }
</script>

<div style={`width: ${NODE.width}px;`} class="center-center" />
