<script lang="ts">
	import { triggerIconMap, type TriggerType } from './utils'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { createEventDispatcher } from 'svelte'

	export let setDropdownWidthToButtonWidth: boolean = false

	const dispatch = createEventDispatcher<{
		addDraftTrigger: TriggerType
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
		{ displayName: 'SQS', action: () => addDraftTrigger('sqs'), icon: triggerIconMap.sqs }
	]

	let triggersButtonWidth = 0

	function addDraftTrigger(type: TriggerType) {
		dispatch('addDraftTrigger', type)
	}
</script>

<DropdownV2
	items={addTriggerItems}
	placement="bottom"
	class={$$props.class}
	customWidth={setDropdownWidthToButtonWidth ? triggersButtonWidth : undefined}
	usePointerDownOutside
>
	<div slot="buttonReplacement" class={$$props.class} bind:clientWidth={triggersButtonWidth}>
		<slot />
	</div>
</DropdownV2>
