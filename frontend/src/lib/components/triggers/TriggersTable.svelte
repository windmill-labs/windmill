<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus, Star, Loader2, Trash, Pen, EllipsisVertical } from 'lucide-svelte'
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
	import { triggerIconMap, type Trigger, type TriggerType } from './utils'
	import type { TriggerContext } from '$lib/components/triggers'

	// Props
	export let path: string
	export let isFlow: boolean = false
	export let selectedTrigger: { path: string; type: string; isDraft?: boolean } | null = null

	// Component state
	let triggers: Trigger[] = []
	let loading = false
	let primaryScheduleExists = false
	let triggersButtonWidth = 0

	const { primarySchedule } = getContext<TriggerContext>('TriggerContext')

	// Event handling
	const dispatch = createEventDispatcher<{
		select: Trigger
		delete: Trigger
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

	const deleteTriggerItems: Item[] = [
		{
			displayName: 'Edit',
			action: () => {
				console.log('edit trigger')
			},
			icon: Pen
		},
		{
			displayName: 'Delete',
			action: () => {
				console.log('delete trigger')
			},
			icon: Trash,
			type: 'delete'
		}
	]

	function addDraftTrigger(type: TriggerType) {
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

<div class="flex flex-col space-y-2 w-full">
	<div class="w-full">
		<DropdownV2
			items={addTriggerItems}
			placement="bottom"
			class="w-full"
			customWidth={triggersButtonWidth}
		>
			<div slot="buttonReplacement" class="w-full" bind:clientWidth={triggersButtonWidth}>
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
								<Star size={10} class="absolute -mt-3 ml-3 text-blue-400" />
							{/if}
						</div>
					</td>
					<td class="py-2 px-2 text-xs">
						<div class="flex items-center justify-between gap-2">
							<div class="flex items-center">
								<span class={trigger.isDraft ? 'text-frost-400 italic' : ''}>
									{trigger.isDraft ? `New ${trigger.type.replace(/s$/, '')} trigger` : trigger.path}
								</span>

								{#if trigger.isPrimary}
									<span
										class="ml-2 bg-blue-50 dark:bg-blue-900/40 px-1.5 py-0.5 rounded text-xs text-blue-700 dark:text-blue-100"
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

							{#if ['schedule', 'http'].includes(trigger.type)}
								{#if trigger.isDraft}
									<Button
										size="xs"
										color="light"
										btnClasses="hover:bg-red-500 hover:text-white bg-transparent"
										startIcon={{ icon: Trash }}
										iconOnly
										on:click={() => deleteDraft(trigger)}
									/>
								{:else}
									<DropdownV2
										items={deleteTriggerItems}
										placement="bottom-end"
										class="w-fit h-fit px-3"
									>
										<svelte:fragment slot="buttonReplacement">
											<EllipsisVertical size={14} />
										</svelte:fragment>
									</DropdownV2>
								{/if}
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
