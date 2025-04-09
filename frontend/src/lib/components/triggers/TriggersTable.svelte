<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		Calendar,
		Route,
		Unplug,
		Database,
		Mail,
		Webhook,
		Plus,
		Star,
		ChevronDown,
		Pen,
		Save
	} from 'lucide-svelte'
	import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
	import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
	import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
	import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
	import type { Item } from '$lib/utils'
	import {
		HttpTriggerService,
		ScheduleService,
		PostgresTriggerService,
		WebsocketTriggerService,
		KafkaTriggerService,
		NatsTriggerService,
		MqttTriggerService,
		SqsTriggerService,
		type Schedule,
		type HttpTrigger,
		type PostgresTrigger,
		type KafkaTrigger
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { createSelect, melt } from '@melt-ui/svelte'
	import type { Trigger } from './utils'

	const {
		elements: { trigger, menu, option },
		states: { open, selected }
	} = createSelect<Trigger>({
		forceVisible: true,
		positioning: {
			placement: 'bottom',
			fitViewport: true,
			sameWidth: true
		},
		onSelectedChange: ({ curr, next }) => {
			if (curr !== next) {
				dispatch('select', next?.value)
			}
			return next
		}
	})

	// Props
	export let path: string
	export let isFlow: boolean = false
	export let canEdit: boolean = false
	export let isEditing: boolean = false

	// Map of trigger kinds to icons
	const triggerIconMap = {
		webhooks: Webhook,
		emails: Mail,
		schedules: Calendar,
		routes: Route, // HTTP
		websockets: Unplug,
		postgres: Database,
		kafka: KafkaIcon,
		nats: NatsIcon,
		mqtt: MqttIcon,
		sqs: AwsIcon,
		primary_schedule: Calendar
	}

	// Component state
	let triggers: {
		type: string
		path: string
		isPrimary?: boolean
		isDraft?: boolean
	}[] = []
	let loading = false

	// Event handling
	const dispatch = createEventDispatcher<{
		select: Trigger | undefined
		save: void
	}>()

	// Dropdown items for adding new triggers
	const addTriggerItems: Item[] = [
		{ displayName: 'Schedule', action: () => addDraftTrigger('schedules'), icon: Calendar },
		{ displayName: 'HTTP', action: () => addDraftTrigger('routes'), icon: Route },
		{ displayName: 'WebSockets', action: () => addDraftTrigger('websockets'), icon: Unplug },
		{ displayName: 'Postgres', action: () => addDraftTrigger('postgres'), icon: Database },
		{ displayName: 'Kafka', action: () => addDraftTrigger('kafka'), icon: KafkaIcon },
		{ displayName: 'NATS', action: () => addDraftTrigger('nats'), icon: NatsIcon },
		{ displayName: 'MQTT', action: () => addDraftTrigger('mqtt'), icon: MqttIcon },
		{ displayName: 'SQS', action: () => addDraftTrigger('sqs'), icon: AwsIcon }
	]

	function addDraftTrigger(type: string) {
		// Remove any existing draft of the same type
		triggers = triggers.filter((t) => !(t.isDraft && t.type === type))

		// Create the new draft trigger
		const newTrigger = {
			type,
			path: path,
			isPrimary: false,
			isDraft: true
		}

		// Add new draft by creating a completely new array to ensure reactivity
		triggers = [...triggers, newTrigger]

		// Select the new draft trigger
		selectTrigger(newTrigger)
	}

	// Select a trigger
	function selectTrigger(trigger: { type: string; path: string; isDraft?: boolean }) {
		$selected = { value: trigger, label: trigger.path }
	}

	// Fetch all triggers
	async function fetchTriggers() {
		loading = true
		try {
			// Store existing draft triggers and any selected state
			const draftTriggers = triggers.filter((t) => t.isDraft)
			const currentSelectedType = $selected?.value?.type

			// Clear existing triggers
			triggers = []

			// Fetch each type of trigger
			await Promise.all([
				fetchSchedules(),
				fetchHttpTriggers(),
				fetchWebsocketTriggers(),
				fetchPostgresTriggers(),
				fetchKafkaTriggers(),
				fetchNatsTriggers(),
				fetchMqttTriggers(),
				fetchSqsTriggers()
			])

			// Add default triggers for webhooks and emails
			const webhookExists = triggers.some((t) => t.type === 'webhooks')
			const emailExists = triggers.some((t) => t.type === 'emails')

			if (!webhookExists) {
				triggers = [...triggers, { type: 'webhooks', path: path, isDraft: false }]
			}

			if (!emailExists) {
				triggers = [...triggers, { type: 'emails', path: path, isDraft: false }]
			}

			// Add back draft triggers
			triggers = [...triggers, ...draftTriggers]

			// If we had a selected draft and that draft still exists, ensure it's selected
			if (currentSelectedType) {
				const draftToSelect = draftTriggers.find((t) => t.type === currentSelectedType && t.isDraft)
				if (draftToSelect) {
					selectTrigger(draftToSelect)
				}
			}
		} catch (error) {
			console.error('Failed to fetch triggers:', error)
		} finally {
			loading = false
		}
	}

	// Fetch schedules
	async function fetchSchedules() {
		if (!$workspaceStore) return

		try {
			const allSchedules: Schedule[] = await ScheduleService.listSchedules({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			// Find primary schedule (matches the path exactly)
			const primarySchedule = allSchedules.find((s) => s.path === path)

			if (primarySchedule) {
				// Add primary schedule
				triggers = [
					...triggers,
					{
						type: 'schedules',
						path: primarySchedule.path,
						isPrimary: true,
						isDraft: false
					}
				]
			}

			// Add other schedules
			const otherSchedules = allSchedules.filter((s) => s.path !== path)

			for (const schedule of otherSchedules) {
				triggers = [
					...triggers,
					{
						type: 'schedules',
						path: schedule.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {}
	}

	// Fetch HTTP triggers
	async function fetchHttpTriggers() {
		if (!$workspaceStore) return

		try {
			const httpTriggers: HttpTrigger[] = await HttpTriggerService.listHttpTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of httpTriggers) {
				triggers = [
					...triggers,
					{
						type: 'routes',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch HTTP triggers:', error)
		}
	}

	// Fetch Websocket triggers
	async function fetchWebsocketTriggers() {
		if (!$workspaceStore) return

		try {
			const wsTriggers = await WebsocketTriggerService.listWebsocketTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of wsTriggers) {
				triggers = [
					...triggers,
					{
						type: 'websockets',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {}
	}

	// Fetch Postgres triggers
	async function fetchPostgresTriggers() {
		if (!$workspaceStore) return

		try {
			const pgTriggers: PostgresTrigger[] = await PostgresTriggerService.listPostgresTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of pgTriggers) {
				triggers = [
					...triggers,
					{
						type: 'postgres',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch Postgres triggers:', error)
		}
	}

	// Fetch Kafka triggers
	async function fetchKafkaTriggers() {
		if (!$workspaceStore) return

		try {
			const kafkaTriggers: KafkaTrigger[] = await KafkaTriggerService.listKafkaTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of kafkaTriggers) {
				triggers = [
					...triggers,
					{
						type: 'kafka',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch Kafka triggers:', error)
		}
	}

	// Fetch NATS triggers
	async function fetchNatsTriggers() {
		if (!$workspaceStore) return

		try {
			const natsTriggers = await NatsTriggerService.listNatsTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of natsTriggers) {
				triggers = [
					...triggers,
					{
						type: 'nats',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch NATS triggers:', error)
		}
	}

	// Fetch MQTT triggers
	async function fetchMqttTriggers() {
		if (!$workspaceStore) return

		try {
			const mqttTriggers = await MqttTriggerService.listMqttTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of mqttTriggers) {
				triggers = [
					...triggers,
					{
						type: 'mqtt',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch MQTT triggers:', error)
		}
	}

	// Fetch SQS triggers
	async function fetchSqsTriggers() {
		if (!$workspaceStore) return

		try {
			const sqsTriggers = await SqsTriggerService.listSqsTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of sqsTriggers) {
				triggers = [
					...triggers,
					{
						type: 'sqs',
						path: trigger.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {
			console.error('Failed to fetch SQS triggers:', error)
		}
	}

	// Fetch triggers on mount and when path changes
	$: if (path && $workspaceStore && !triggers.length) {
		fetchTriggers()
	}

	$: triggerOptions = triggers.map((t) => ({
		id: t.type + '_' + t.path,
		trigger: t
	}))
</script>

{#snippet triggerRow(trigger: {
	type: string
	path: string
	isDraft?: boolean
	isPrimary?: boolean
})}
	<div class="flex flex-row w-full gap-2 py-1">
		<div class="flex justify-center items-center">
			<svelte:component
				this={triggerIconMap[trigger.type]}
				size={16}
				class={trigger.isDraft ? 'text-frost-400' : 'text-tertiary'}
			/>

			{#if trigger.isPrimary}
				<Star size={10} class="absolute -mt-3 ml-3 text-yellow-400" />
			{/if}
		</div>

		<div class="flex items-center">
			<span class={trigger.isDraft ? 'text-frost-400 italic' : ''}>
				{trigger.isDraft ? `New ${trigger.type.replace(/s$/, '')} trigger` : trigger.path}
			</span>

			{#if trigger.isPrimary}
				<span
					class="ml-2 text-2xs bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-100 px-1.5 py-0.5 rounded"
				>
					Primary
				</span>
			{/if}

			{#if trigger.isDraft}
				<span
					class="ml-2 text-2xs bg-frost-100 dark:bg-frost-900 text-frost-800 dark:text-frost-100 px-1.5 py-0.5 rounded"
				>
					Draft
				</span>
			{/if}
		</div>
	</div>
{/snippet}

<div class="flex flex-row gap-2">
	<button
		class="grow min-w-40 rounded-md border flex flex-row items-center justify-between px-4"
		use:melt={$trigger}
	>
		{#if $selected?.value}
			{@render triggerRow($selected?.value)}
		{:else}
			<span>Select a trigger</span>
		{/if}
		<ChevronDown size={16} class="text-secondary" />
	</button>
	{#if $open}
		<div
			use:melt={$menu}
			class="z-[1000] max-h-[300px] overflow-y-auto rounded-lg shadow-lg bg-surface"
		>
			{#each triggerOptions as opt (opt.id)}
				<div
					use:melt={$option({ value: opt.trigger, label: opt.trigger.path })}
					class="cursor-pointer text-secondary hover:bg-surface-hover data-[highlighted]:bg-surface-hover data-[disabled]:opacity-50 px-4"
				>
					{@render triggerRow(opt.trigger)}
				</div>
			{/each}
		</div>
	{/if}

	<div class="flex flex-row gap-2">
		{#if canEdit && !isEditing}
			<Button
				size="sm"
				color="light"
				variant="border"
				startIcon={{ icon: Pen }}
				on:click={() => (isEditing = true)}>Edit</Button
			>
		{:else if canEdit && isEditing}
			<div class="flex flex-row gap-2">
				<Button
					size="sm"
					color="blue"
					startIcon={{ icon: Save }}
					on:click={() => {
						isEditing = false
						dispatch('save')
					}}>Save</Button
				>
			</div>
		{/if}
		<DropdownV2 items={addTriggerItems} placement="bottom-end" class="w-fit">
			<div slot="buttonReplacement">
				<Button size="sm" color="blue" startIcon={{ icon: Plus }} nonCaptureEvent>
					<span>Add trigger</span>
				</Button>
			</div>
		</DropdownV2>
	</div>
</div>
