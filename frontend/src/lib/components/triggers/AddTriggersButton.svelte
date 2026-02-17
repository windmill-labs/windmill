<script lang="ts">
	import { triggerIconMap, type TriggerType } from './utils'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { SchedulePollIcon } from '$lib/components/icons'
	import type { Placement } from '@floating-ui/core'
	import { isCloudHosted } from '$lib/cloud'
	import { CloudOff } from 'lucide-svelte'
	import { isServiceAvailable } from './native/utils'
	import { workspaceStore } from '$lib/stores'

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
		placement = 'bottom-start',
		isEditor = false,
		onAddDraftTrigger,
		onAddScheduledPoll,
		onClose
	}: Props = $props()

	let dropdown: DropdownV2 | undefined

	const cloudHosted = isCloudHosted()
	let nextcloudAvailable = $state(false)
	let googleAvailable = $state(false)

	async function setNextcloudState() {
		nextcloudAvailable = await isServiceAvailable('nextcloud', $workspaceStore!)
	}

	async function setGoogleState() {
		googleAvailable = await isServiceAvailable('google', $workspaceStore!)
	}

	setNextcloudState()
	setGoogleState()

	const addTriggerItems = $derived(
		[
			{
				displayName: 'Schedule',
				action: () => onAddDraftTrigger?.('schedule'),
				icon: triggerIconMap.schedule
			},
			{ displayName: 'HTTP', action: () => onAddDraftTrigger?.('http'), icon: triggerIconMap.http },
			{
				displayName: 'WebSocket',
				action: () => onAddDraftTrigger?.('websocket'),
				icon: triggerIconMap.websocket,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'Postgres',
				action: () => onAddDraftTrigger?.('postgres'),
				icon: triggerIconMap.postgres,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'Kafka',
				action: () => onAddDraftTrigger?.('kafka'),
				icon: triggerIconMap.kafka,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'NATS',
				action: () => onAddDraftTrigger?.('nats'),
				icon: triggerIconMap.nats,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'MQTT',
				action: () => onAddDraftTrigger?.('mqtt'),
				icon: triggerIconMap.mqtt,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'SQS',
				action: () => onAddDraftTrigger?.('sqs'),
				icon: triggerIconMap.sqs,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'GCP Pub/Sub',
				action: () => onAddDraftTrigger?.('gcp'),
				icon: triggerIconMap.gcp,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'Email',
				action: () => onAddDraftTrigger?.('email'),
				icon: triggerIconMap.email,
				extra: cloudHosted ? extra : undefined
			},
			{
				displayName: 'Scheduled Poll',
				action: (e) => {
					onAddDraftTrigger?.('poll')
					onAddScheduledPoll?.()
				},
				icon: SchedulePollIcon,
				hidden: !isEditor
			},
			{
				displayName: 'Nextcloud',
				action: () => onAddDraftTrigger?.('nextcloud'),
				icon: triggerIconMap.nextcloud,
				hidden: !nextcloudAvailable
			},
			{
				displayName: 'Google',
				action: () => onAddDraftTrigger?.('google'),
				icon: triggerIconMap.google,
				hidden: !googleAvailable
			}
		].filter((item) => !item.hidden)
	)

	let triggersButtonWidth = $state(0)

	export function close() {
		dropdown?.close()
	}
</script>

{#snippet extra()}
	<p
		class="text-xs text-yellow-700 dark:text-yellow-100/90 bg-yellow-50 dark:bg-yellow-900/40 rounded-md p-1 px-2 -my-1"
		title="Disabled in multi-tenant cloud"
	>
		<CloudOff size={14} />
	</p>
{/snippet}

<DropdownV2
	enableFlyTransition
	bind:this={dropdown}
	items={addTriggerItems}
	{placement}
	class={className}
	customWidth={setDropdownWidthToButtonWidth ? triggersButtonWidth : undefined}
	usePointerDownOutside
	on:close={() => onClose?.()}
>
	{#snippet buttonReplacement()}
		<div class={className} bind:clientWidth={triggersButtonWidth}>
			{@render children?.()}
		</div>
	{/snippet}
</DropdownV2>
