import { Webhook, Mail, Calendar, Route, Unplug, Database } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import type { CaptureTriggerKind } from '$lib/gen/types.gen'
import type { Writable } from 'svelte/store'
import { get } from 'svelte/store'
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
import type { ScheduleTrigger } from '$lib/components/triggers'
import { canWrite } from '$lib/utils'
import type { UserExt } from '$lib/stores'

export type TriggerType =
	| 'webhook'
	| 'email'
	| 'schedule'
	| 'http'
	| 'websocket'
	| 'postgres'
	| 'kafka'
	| 'nats'
	| 'mqtt'
	| 'sqs'

export type Trigger = {
	path: string
	type: TriggerType
	isDraft?: boolean
	isPrimary?: boolean
	canWrite?: boolean
}

// Map of trigger kinds to icons
export const triggerIconMap = {
	webhook: Webhook,
	email: Mail,
	schedule: Calendar,
	http: Route,
	websocket: Unplug,
	postgres: Database,
	kafka: KafkaIcon,
	nats: NatsIcon,
	mqtt: MqttIcon,
	sqs: AwsIcon,
	primary_schedule: Calendar
}

/**
 * Converts a TriggerType to a CaptureTriggerKind when a mapping exists
 * @param triggerType The trigger type to convert
 * @returns The corresponding CaptureTriggerKind or undefined if no mapping exists
 */
export function triggerTypeToCaptureKind(triggerType: TriggerType): CaptureTriggerKind | undefined {
	// The types that don't map to CaptureTriggerKind
	const nonCaptureTriggerTypes = ['schedule', 'primary_schedule']

	if (nonCaptureTriggerTypes.includes(triggerType)) {
		return undefined
	}

	// Since we've filtered out non-capturable types, we can safely assert this as CaptureTriggerKind
	return triggerType as CaptureTriggerKind
}

/**
 * Add a draft trigger to the store
 */
export function addDraftTrigger(
	triggersStore: Writable<Trigger[]>,
	type: TriggerType,
	primaryScheduleExists: boolean
): Trigger {
	// Remove any existing draft of the same type from the store
	const currentTriggers = get(triggersStore)
	triggersStore.set(currentTriggers.filter((t) => !(t.isDraft && t.type === type)))

	// Create the new draft trigger
	const newTrigger = {
		type,
		path: '',
		isPrimary: type === 'schedule' && !primaryScheduleExists,
		isDraft: true
	}

	// Add new draft to the store
	triggersStore.update((triggers) => [...triggers, newTrigger])

	return newTrigger
}

/**
 * Delete a draft trigger from the store
 */
export function deleteDraft(
	triggersStore: Writable<Trigger[]>,
	trigger: Trigger | undefined
): void {
	if (!trigger) return

	triggersStore.update((triggers) =>
		triggers.filter(
			(t) => !(t.type === trigger.type && t.isDraft && t.isPrimary === trigger.isPrimary)
		)
	)
}

/**
 * Fetch all types of triggers
 */
export async function fetchTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	primarySchedule: ScheduleTrigger | undefined | false = undefined,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return

	// Store existing draft triggers
	const currentTriggers = get(triggersStore)
	const draftTriggers = currentTriggers.filter((t) => t.isDraft)

	// Fetch each type of trigger
	await Promise.all([
		fetchSchedules(triggersStore, workspaceId, path, isFlow, primarySchedule),
		fetchHttpTriggers(triggersStore, workspaceId, path, isFlow, user),
		fetchWebsocketTriggers(triggersStore, workspaceId, path, isFlow),
		fetchPostgresTriggers(triggersStore, workspaceId, path, isFlow),
		fetchKafkaTriggers(triggersStore, workspaceId, path, isFlow),
		fetchNatsTriggers(triggersStore, workspaceId, path, isFlow),
		fetchMqttTriggers(triggersStore, workspaceId, path, isFlow),
		fetchSqsTriggers(triggersStore, workspaceId, path, isFlow)
	])

	// Add back draft triggers
	triggersStore.update((triggers) => [...triggers, ...draftTriggers])
}

/**
 * Fetch schedule triggers
 */
export async function fetchSchedules(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	primarySchedule: ScheduleTrigger | undefined | false = undefined
): Promise<boolean> {
	if (!workspaceId) return false
	try {
		const allSchedules: Schedule[] = await ScheduleService.listSchedules({
			workspace: workspaceId,
			path,
			isFlow
		})

		// Remove existing schedules
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'schedule')))

		// Find primary schedule (matches the path exactly)
		const deployedPrimarySchedule = allSchedules.find((s) => s.path === path)
		let primaryScheduleExists = false

		if (deployedPrimarySchedule) {
			// Add primary schedule
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'schedule',
					path: deployedPrimarySchedule.path,
					isPrimary: true,
					isDraft: false
				}
			])
			primaryScheduleExists = true
		} else if (primarySchedule && primarySchedule) {
			// Primary schedule in the store is not deployed, so we add it
			primaryScheduleExists = true
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'schedule',
					path: path,
					isPrimary: true,
					isDraft: false
				}
			])
		}

		// Add other schedules
		const otherSchedules = allSchedules.filter((s) => s.path !== path)

		for (const schedule of otherSchedules) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'schedule',
					path: schedule.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}

		return primaryScheduleExists
	} catch (error) {
		console.error('Failed to fetch schedules:', error)
		return false
	}
}

/**
 * Fetch HTTP triggers
 */
export async function fetchHttpTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		// Remove existing HTTP triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'http')))

		const httpTriggers: HttpTrigger[] = await HttpTriggerService.listHttpTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of httpTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'http',
					path: trigger.path,
					isPrimary: false,
					isDraft: false,
					canWrite: canWrite(trigger.path, trigger.extra_perms, user)
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch HTTP triggers:', error)
	}
}

/**
 * Fetch Websocket triggers
 */
export async function fetchWebsocketTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing websocket triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'websocket')))

		const wsTriggers = await WebsocketTriggerService.listWebsocketTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of wsTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'websocket',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch Websocket triggers:', error)
	}
}

/**
 * Fetch Postgres triggers
 */
export async function fetchPostgresTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing postgres triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'postgres')))

		const pgTriggers: PostgresTrigger[] = await PostgresTriggerService.listPostgresTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of pgTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'postgres',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch Postgres triggers:', error)
	}
}

/**
 * Fetch Kafka triggers
 */
export async function fetchKafkaTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing kafka triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'kafka')))

		const kafkaTriggers: KafkaTrigger[] = await KafkaTriggerService.listKafkaTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of kafkaTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'kafka',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch Kafka triggers:', error)
	}
}

/**
 * Fetch NATS triggers
 */
export async function fetchNatsTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing NATS triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'nats')))

		const natsTriggers = await NatsTriggerService.listNatsTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of natsTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'nats',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch NATS triggers:', error)
	}
}

/**
 * Fetch MQTT triggers
 */
export async function fetchMqttTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing MQTT triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'mqtt')))

		const mqttTriggers = await MqttTriggerService.listMqttTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of mqttTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'mqtt',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch MQTT triggers:', error)
	}
}

/**
 * Fetch SQS triggers
 */
export async function fetchSqsTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string,
	path: string,
	isFlow: boolean
): Promise<void> {
	try {
		// Remove existing SQS triggers for this path
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'sqs')))

		const sqsTriggers = await SqsTriggerService.listSqsTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of sqsTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'sqs',
					path: trigger.path,
					isPrimary: false,
					isDraft: false
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch SQS triggers:', error)
	}
}
