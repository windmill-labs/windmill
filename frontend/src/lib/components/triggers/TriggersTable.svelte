<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { createEventDispatcher } from 'svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus, Star, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
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
	import { triggerIconMap } from './utils'
	// Props
	export let path: string
	export let isFlow: boolean = false
	export let selectedTrigger: { path: string; type: string; isDraft?: boolean } | null = null

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
		select: { path: string; type: string; isDraft?: boolean }
	}>()

	// Dropdown items for adding new triggers
	const addTriggerItems: Item[] = [
		{
			displayName: 'Schedule',
			action: () => addDraftTrigger('schedules'),
			icon: triggerIconMap.schedules
		},
		{ displayName: 'HTTP', action: () => addDraftTrigger('routes'), icon: triggerIconMap.routes },
		{
			displayName: 'WebSockets',
			action: () => addDraftTrigger('websockets'),
			icon: triggerIconMap.websockets
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
		dispatch('select', {
			path: trigger.path,
			type: trigger.type,
			isDraft: trigger.isDraft
		})
	}

	// Fetch all triggers
	async function fetchTriggers() {
		loading = true
		try {
			// Store existing draft triggers and any selected state
			const draftTriggers = triggers.filter((t) => t.isDraft)
			const currentSelectedType = selectedTrigger?.type

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
</script>

<div class="flex flex-col space-y-2 w-full">
	<div class="w-full">
		<DropdownV2 items={addTriggerItems} placement="bottom" class="w-full">
			<div slot="buttonReplacement" class="w-full">
				<Button
					size="xs"
					color="blue"
					startIcon={{ icon: Plus }}
					nonCaptureEvent
					btnClasses="w-full justify-center"
				>
					<span>Add trigger</span>
				</Button>
			</div>
		</DropdownV2>
	</div>

	<DataTable {loading} size="sm" tableFixed={true}>
		<tbody>
			{#each triggers as trigger}
				<tr
					class={twMerge(
						'hover:bg-surface-hover cursor-pointer border-b border-t border-transparent',
						selectedTrigger &&
							selectedTrigger.path === trigger.path &&
							selectedTrigger.type === trigger.type &&
							selectedTrigger.isDraft === trigger.isDraft
							? 'bg-surface-hover '
							: ''
					)}
					on:click={() => selectTrigger(trigger)}
				>
					<td class="w-12 text-center py-2 px-2">
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
					</td>
					<td class="py-2 px-2 text-xs">
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
					</td>
				</tr>
			{/each}

			{#if !loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm"> No triggers found </td>
				</tr>
			{/if}
			{#if loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm">
						<div class="flex justify-center items-center gap-2">
							<Loader2 class="animate-spin" size={16} />
							<span>Loading triggers...</span>
						</div>
					</td>
				</tr>
			{/if}
		</tbody>
	</DataTable>
</div>
