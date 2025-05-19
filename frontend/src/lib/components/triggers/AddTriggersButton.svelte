<script lang="ts">
	import { triggerIconMap, type TriggerType } from './utils'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { SchedulePollIcon } from '$lib/components/icons'
	import type { Placement } from '@floating-ui/core'

	interface Props {
		setDropdownWidthToButtonWidth?: boolean
		children?: import('svelte').Snippet
		triggerScriptPicker?: import('svelte').Snippet | undefined
		class?: string
		placement?: Placement
		isEditor?: boolean
		onAddDraftTrigger?: (type: TriggerType) => void
		onAddScheduledPoll?: () => void
		onClose?: () => void
	}

	let {
		setDropdownWidthToButtonWidth = false,
		children,
		class: className,
		triggerScriptPicker,
		placement = 'bottom',
		isEditor = false,
		onAddDraftTrigger,
		onAddScheduledPoll,
		onClose
	}: Props = $props()

	let dropdown: DropdownV2 | undefined

	// Dropdown items for adding new triggers
	const addTriggerItems: Item[] = [
		{
			displayName: 'Schedule',
			action: () => onAddDraftTrigger?.('schedule'),
			icon: triggerIconMap.schedule
		},
		{ displayName: 'HTTP', action: () => onAddDraftTrigger?.('http'), icon: triggerIconMap.http },
		{
			displayName: 'WebSocket',
			action: () => onAddDraftTrigger?.('websocket'),
			icon: triggerIconMap.websocket
		},
		{
			displayName: 'Postgres',
			action: () => onAddDraftTrigger?.('postgres'),
			icon: triggerIconMap.postgres
		},
		{
			displayName: 'Kafka',
			action: () => onAddDraftTrigger?.('kafka'),
			icon: triggerIconMap.kafka
		},
		{ displayName: 'NATS', action: () => onAddDraftTrigger?.('nats'), icon: triggerIconMap.nats },
		{ displayName: 'MQTT', action: () => onAddDraftTrigger?.('mqtt'), icon: triggerIconMap.mqtt },
		{ displayName: 'SQS', action: () => onAddDraftTrigger?.('sqs'), icon: triggerIconMap.sqs },
		{
			displayName: 'GCP Pub/Sub',
			action: () => onAddDraftTrigger?.('gcp'),
			icon: triggerIconMap.gcp
		},
		{
			displayName: 'Scheduled Poll',
			action: (e) => {
				e.preventDefault()
				onAddDraftTrigger?.('poll')
				onAddScheduledPoll?.()
			},
			icon: SchedulePollIcon,
			hidden: !isEditor
		}
	].filter((item) => !item.hidden)

	let triggersButtonWidth = $state(0)

	export function close() {
		dropdown?.close()
	}
</script>

<DropdownV2
	bind:this={dropdown}
	items={addTriggerItems}
	{placement}
	class={className}
	customWidth={setDropdownWidthToButtonWidth ? triggersButtonWidth : undefined}
	usePointerDownOutside
	customMenu={!!triggerScriptPicker}
	on:close={() => onClose?.()}
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
