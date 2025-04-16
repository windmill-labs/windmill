<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
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
	import { type Trigger, type TriggerType } from './utils'
	import type { TriggerContext } from '$lib/components/triggers'

	// Props
	export let path: string
	export let isFlow: boolean = false
	export let selectedTrigger: { path: string; type: string; isDraft?: boolean } | null = null
	export let triggers: Trigger[] = []

	// Component state
	let loading = false
	let primaryScheduleExists = false

	const { primarySchedule } = getContext<TriggerContext>('TriggerContext')

	// Event handling
	const dispatch = createEventDispatcher<{
		select: Trigger
		delete: Trigger
	}>()

	export function addDraftTrigger(type: TriggerType) {
		// Remove any existing draft of the same type
		triggers = triggers.filter((t) => !(t.isDraft && t.type === type))

		// Create the new draft trigger
		const newTrigger = {
			type,
			path: '',
			isPrimary: type === 'schedule' && !primaryScheduleExists,
			isDraft: true
		}

		// Add new draft by creating a completely new array to ensure reactivity
		triggers = [...triggers, newTrigger]

		// Select the new draft trigger
		selectTrigger(newTrigger)
	}

	// Select a trigger
	function selectTrigger(trigger: Trigger) {
		dispatch('select', trigger)
	}

	// Fetch all triggers
	async function fetchTriggers() {
		loading = true
		try {
			// Store existing draft triggers and any selected state
			const draftTriggers = triggers.filter((t) => t.isDraft)
			const currentSelectedType = selectedTrigger?.type

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
			const webhookExists = triggers.some((t) => t.type === 'webhook')
			const emailExists = triggers.some((t) => t.type === 'email')

			if (!webhookExists) {
				triggers = [...triggers, { type: 'webhook', path: path, isDraft: false }]
			}

			if (!emailExists) {
				triggers = [...triggers, { type: 'email', path: path, isDraft: false }]
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
	export async function fetchSchedules() {
		if (!$workspaceStore) return

		try {
			const allSchedules: Schedule[] = await ScheduleService.listSchedules({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			// Clear existing schedules
			triggers = triggers.filter((t) => t.type !== 'schedule')

			// Find primary schedule (matches the path exactly)
			const deployedPrimarySchedule = allSchedules.find((s) => s.path === path)

			if (deployedPrimarySchedule) {
				// Add primary schedule
				triggers = [
					...triggers,
					{
						type: 'schedule',
						path: deployedPrimarySchedule.path,
						isPrimary: true,
						isDraft: false
					}
				]
				primaryScheduleExists = true
			} else if ($primarySchedule) {
				primaryScheduleExists = true
				triggers = [
					...triggers,
					{
						type: 'schedule',
						path: path,
						isPrimary: true,
						isDraft: false
					}
				]
			} else {
				primaryScheduleExists = false
			}

			// Add other schedules
			const otherSchedules = allSchedules.filter((s) => s.path !== path)

			for (const schedule of otherSchedules) {
				triggers = [
					...triggers,
					{
						type: 'schedule',
						path: schedule.path,
						isPrimary: false,
						isDraft: false
					}
				]
			}
		} catch (error) {}
	}

	// Fetch HTTP triggers
	export async function fetchHttpTriggers() {
		if (!$workspaceStore) return

		try {
			triggers = triggers.filter((t) => t.type !== 'http') // Remove any existing routes
			const httpTriggers: HttpTrigger[] = await HttpTriggerService.listHttpTriggers({
				workspace: $workspaceStore,
				path,
				isFlow
			})

			for (const trigger of httpTriggers) {
				triggers = [
					...triggers,
					{
						type: 'http',
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
						type: 'websocket',
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

	export function deleteDraft(trigger: Trigger | undefined, keepSelection: boolean = false) {
		if (!trigger) return
		triggers = triggers.filter(
			(t) => !(t.type === trigger.type && t.isDraft && t.isPrimary === trigger.isPrimary)
		)
		if (!keepSelection) {
			selectTrigger(triggers[-1])
		}
	}
</script>
