import { Webhook, Mail, Calendar, Route, Unplug, Database } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
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
	GcpTriggerService,
	type Schedule,
	type HttpTrigger,
	type PostgresTrigger,
	type KafkaTrigger,
	type GcpTrigger
} from '$lib/gen'
import type { ScheduleTrigger } from '$lib/components/triggers'
import { canWrite } from '$lib/utils'
import type { UserExt } from '$lib/stores'
import SchedulePollIcon from '../icons/SchedulePollIcon.svelte'
import { type TriggerKind } from '$lib/components/triggers'

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
	| 'gcp'
	| 'poll'

export type Trigger = {
	type: TriggerType
	path?: string
	isDraft?: boolean
	isPrimary?: boolean
	canWrite?: boolean
	id?: string
	draftConfig?: Record<string, any>
	captureConfig?: Record<string, any>
	saveCb?: () => void
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
	gcp: GoogleCloudIcon,
	primary_schedule: Calendar,
	poll: SchedulePollIcon
}

export function isEqual(a: Trigger, b: Trigger): boolean {
	return a.path === b.path && a.type === b.type && a.isDraft === b.isDraft && a.id === b.id
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
 * Generate a unique ID for a draft trigger
 */
function generateDraftId(): string {
	return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15)
}

/**
 * Add a draft trigger to the store
 */
export function addDraftTrigger(
	triggersStore: Writable<Trigger[]>,
	type: TriggerType,
	path?: string
): Trigger {
	const currentTriggers = get(triggersStore)

	const primaryScheduleExists = currentTriggers.some((t) => t.type === 'schedule' && t.isPrimary)

	// Create the new draft trigger
	const draftId = generateDraftId()
	const newTrigger = {
		id: draftId,
		type,
		path,
		isPrimary: type === 'schedule' && !primaryScheduleExists,
		isDraft: true
	}

	// Add new draft to the store
	triggersStore.update((triggers) => [...triggers, newTrigger])

	return newTrigger
}

export function setDraftToDeployedTrigger(
	triggersStore: Writable<Trigger[]>,
	trigger: Trigger,
	draftConfig: Record<string, any>
): void {
	let draftId = trigger.id
	if (!draftId) {
		draftId = generateDraftId()
	}
	if (trigger.isDraft) {
		triggersStore.update((triggers) =>
			triggers.map((t) =>
				t.id === draftId ? { ...t, deployedPath: trigger.path, draftConfig, id: trigger.id } : t
			)
		)
	} else {
		triggersStore.update((triggers) =>
			triggers.map((t) =>
				t.path === trigger.path && t.type === trigger.type ? { ...t, draftConfig } : t
			)
		)
	}
}

/**
 * Delete a draft trigger from the store
 */
export function deleteDraft(triggersStore: Writable<Trigger[]>, draftId: string): void {
	triggersStore.update((triggers) => triggers.filter((t) => t.id !== draftId))
}

export function updateDraftConfig(
	triggersStore: Writable<Trigger[]>,
	trigger: Trigger,
	draftConfig: Record<string, any> | undefined
): void {
	if (trigger.isDraft) {
		triggersStore.update((triggers) =>
			triggers.map((t) =>
				t.id === trigger.id
					? {
							...t,
							draftConfig,
							...(draftConfig === undefined ? { saveCb: undefined } : {})
						}
					: t
			)
		)
	} else {
		triggersStore.update((triggers) =>
			triggers.map((t) =>
				t.path === trigger.path && t.type === trigger.type
					? {
							...t,
							draftConfig,
							...(draftConfig === undefined ? { saveCb: undefined } : {})
						}
					: t
			)
		)
	}
}

export function setCaptureConfig(
	triggersStore: Writable<Trigger[]>,
	draftId: string,
	captureConfig: Record<string, any>
): void {
	triggersStore.update((triggers) =>
		triggers.map((t) => (t.id === draftId ? { ...t, captureConfig } : t))
	)
}

export function setSaveCallback(
	triggersStore: Writable<Trigger[]>,
	trigger: Trigger,
	saveCb: () => void
) {
	if (trigger.isDraft) {
		triggersStore.update((triggers) =>
			triggers.map((t) => (t.id === trigger.id ? { ...t, saveCb } : t))
		)
	} else {
		triggersStore.update((triggers) =>
			triggers.map((t) =>
				t.path === trigger.path && t.type === trigger.type ? { ...t, saveCb } : t
			)
		)
	}
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

	// Fetch each type of trigger
	await Promise.all([
		fetchSchedules(triggersStore, workspaceId, path, isFlow, primarySchedule),
		fetchHttpTriggers(triggersStore, workspaceId, path, isFlow, user),
		fetchWebsocketTriggers(triggersStore, workspaceId, path, isFlow, user),
		fetchPostgresTriggers(triggersStore, workspaceId, path, isFlow, user),
		fetchKafkaTriggers(triggersStore, workspaceId, path, isFlow, user),
		fetchNatsTriggers(triggersStore, workspaceId, path, isFlow),
		fetchMqttTriggers(triggersStore, workspaceId, path, isFlow),
		fetchSqsTriggers(triggersStore, workspaceId, path, isFlow),
		fetchGcpTriggers(triggersStore, workspaceId, path, isFlow, user)
	])
}

function updateTriggers(
	triggersStore: Writable<Trigger[]>,
	remoteTriggers: any[],
	type: TriggerType,
	user: UserExt | undefined = undefined
): void {
	const currentTriggers = get(triggersStore)
	// Identify triggers with draftConfig to preserve
	const configuredTriggers = currentTriggers.filter(
		(t) => t.type === type && !t.isDraft && t.draftConfig
	)

	const configMap = new Map<string, { draftConfig: Record<string, any>; saveCb: () => void }>()

	configuredTriggers.forEach((t) => {
		configMap.set(t.path ?? '', { draftConfig: t.draftConfig!, saveCb: t.saveCb! })
	})

	const backendTriggers = remoteTriggers.map((trigger) => {
		const { draftConfig, saveCb } = configMap.get(trigger.path) ?? {}
		return {
			type: type as TriggerType,
			path: trigger.path,
			isPrimary: false,
			isDraft: false,
			canWrite: canWrite(trigger.path, trigger.extra_perms, user),
			draftConfig: draftConfig,
			saveCb: saveCb
		}
	})

	triggersStore.update((triggers) => {
		const filteredTriggers = triggers.filter((t) => t.type !== type || t.isDraft)
		return [...filteredTriggers, ...backendTriggers]
	})
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

		// Remove existing schedules except for draft schedules
		triggersStore.update((triggers) =>
			triggers.filter((t) => !(t.type === 'schedule' && !t.isDraft))
		)

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
		const httpTriggers: HttpTrigger[] = await HttpTriggerService.listHttpTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		updateTriggers(triggersStore, httpTriggers, 'http', user)
	} catch (error) {
		console.error('Failed to fetch HTTP triggers:', error)
	}
}

/**
 * Fetch Websocket triggers
 */
export async function fetchWebsocketTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const wsTriggers = await WebsocketTriggerService.listWebsocketTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		updateTriggers(triggersStore, wsTriggers, 'websocket', user)
	} catch (error) {
		console.error('Failed to fetch Websocket triggers:', error)
	}
}

/**
 * Fetch Postgres triggers
 */
export async function fetchPostgresTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const pgTriggers: PostgresTrigger[] = await PostgresTriggerService.listPostgresTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		updateTriggers(triggersStore, pgTriggers, 'postgres', user)
	} catch (error) {
		console.error('Failed to fetch Postgres triggers:', error)
	}
}

/**
 * Fetch Kafka triggers
 */
export async function fetchKafkaTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const kafkaTriggers: KafkaTrigger[] = await KafkaTriggerService.listKafkaTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		updateTriggers(triggersStore, kafkaTriggers, 'kafka', user)
	} catch (error) {
		console.error('Failed to fetch Kafka triggers:', error)
	}
}

/**
 * Fetch NATS triggers
 */
export async function fetchNatsTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	try {
		if (!workspaceId) return
		// Remove existing NATS triggers for this path except for draft triggers
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'nats' && !t.isDraft)))

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
					isDraft: false,
					canWrite: canWrite(trigger.path, trigger.extra_perms, user)
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
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	try {
		if (!workspaceId) return
		// Remove existing MQTT triggers for this path except for draft triggers
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'mqtt' && !t.isDraft)))

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
					isDraft: false,
					canWrite: canWrite(trigger.path, trigger.extra_perms, user)
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
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		// Remove existing SQS triggers for this path except for draft triggers
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'sqs' && !t.isDraft)))

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
					isDraft: false,
					canWrite: canWrite(trigger.path, trigger.extra_perms, user)
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch SQS triggers:', error)
	}
}

/**
 * Fetch GCP Pub/Sub triggers
 */
export async function fetchGcpTriggers(
	triggersStore: Writable<Trigger[]>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		// Remove existing GCP triggers for this path except for draft triggers
		triggersStore.update((triggers) => triggers.filter((t) => !(t.type === 'gcp' && !t.isDraft)))

		const gcpTriggers: GcpTrigger[] = await GcpTriggerService.listGcpTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})

		for (const trigger of gcpTriggers) {
			triggersStore.update((triggers) => [
				...triggers,
				{
					type: 'gcp',
					path: trigger.path,
					isPrimary: false,
					isDraft: false,
					canWrite: canWrite(trigger.path, trigger.extra_perms, user)
				}
			])
		}
	} catch (error) {
		console.error('Failed to fetch GCP Pub/Sub triggers:', error)
	}
}

// TODO: Remove this once we've migrated all the trigger kinds to the new TriggerType enum
export function triggerKindToTriggerType(kind: TriggerKind): TriggerType | undefined {
	switch (kind) {
		case 'webhooks':
			return 'webhook'
		case 'emails':
			return 'email'
		case 'schedules':
			return 'schedule'
		case 'routes':
			return 'http'
		case 'websockets':
			return 'websocket'
		case 'postgres':
			return 'postgres'
		case 'kafka':
			return 'kafka'
		case 'nats':
			return 'nats'
		case 'mqtt':
			return 'mqtt'
		case 'sqs':
			return 'sqs'
		case 'gcp':
			return 'gcp'
		case 'scheduledPoll':
			return 'poll'
		default:
			throw new Error(`Unknown TriggerKind: ${kind}`)
	}
}
