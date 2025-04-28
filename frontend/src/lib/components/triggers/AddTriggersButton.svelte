<script lang="ts">
	import { triggerIconMap, type TriggerType } from './utils'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { createEventDispatcher } from 'svelte'
	import { SchedulePollIcon } from '$lib/components/icons'
	import type { Placement } from '@floating-ui/core'

	interface Props {
		setDropdownWidthToButtonWidth?: boolean
		children?: import('svelte').Snippet
		triggerScriptPicker?: import('svelte').Snippet | undefined
		class?: string
		placement?: Placement
	}

	let {
		setDropdownWidthToButtonWidth = false,
		children,
		class: className,
		triggerScriptPicker,
		placement = 'bottom'
	}: Props = $props()

	const dispatch = createEventDispatcher<{
		addDraftTrigger: TriggerType
		addScheduledPoll: void
	}>()

	// Dropdown items for adding new triggers
	const addTriggerItems: Item[] = [
		{
			displayName: 'Schedule',
			action: () => addDraftTrigger('schedule'),
			icon: triggerIconMap.schedule
		},
		{ displayName: 'HTTP', action: () => addDraftTrigger('http'), icon: triggerIconMap.http },
		{
			displayName: 'WebSockets',
			action: () => addDraftTrigger('websocket'),
			icon: triggerIconMap.websocket
		},
		{
			displayName: 'Postgres',
			action: () => addDraftTrigger('postgres'),
			icon: triggerIconMap.postgres
		},
		{ displayName: 'Kafka', action: () => addDraftTrigger('kafka'), icon: triggerIconMap.kafka },
		{ displayName: 'NATS', action: () => addDraftTrigger('nats'), icon: triggerIconMap.nats },
		{ displayName: 'MQTT', action: () => addDraftTrigger('mqtt'), icon: triggerIconMap.mqtt },
		{ displayName: 'SQS', action: () => addDraftTrigger('sqs'), icon: triggerIconMap.sqs },
		{ displayName: 'GCP Pub/Sub', action: () => addDraftTrigger('gcp'), icon: triggerIconMap.gcp },
		{
			displayName: 'Scheduled Poll',
			action: (e) => {
				e.preventDefault()
				addDraftTrigger('poll')
				dispatch('addScheduledPoll')
			},
			icon: SchedulePollIcon
		}
	]

	let triggersButtonWidth = $state(0)

	function addDraftTrigger(type: TriggerType) {
		dispatch('addDraftTrigger', type)
	}
</script>

<DropdownV2
	items={addTriggerItems}
	{placement}
	class={className}
	customWidth={setDropdownWidthToButtonWidth ? triggersButtonWidth : undefined}
	usePointerDownOutside
	customMenu={!!triggerScriptPicker}
	on:close
	on:open
>
	{#snippet buttonReplacement()}
		<div class={className} bind:clientWidth={triggersButtonWidth}>
			{@render children?.()}
		</div>
	{/snippet}
	{#snippet menu()}
		{@render triggerScriptPicker?.()}
	{/snippet}
</DropdownV2>
