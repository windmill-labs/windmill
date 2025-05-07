import { Webhook, Mail, Calendar, Route, Unplug, Database, Terminal } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
import type { CaptureTriggerKind, TriggersCount } from '$lib/gen/types.gen'
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
import { canWrite, formatCron } from '$lib/utils'
import type { UserExt } from '$lib/stores'
import SchedulePollIcon from '../icons/SchedulePollIcon.svelte'
import { type TriggerKind } from '$lib/components/triggers'
import { saveScheduleFromCfg } from '$lib/components/flows/scheduleUtils'
import { saveHttpRouteFromCfg } from './http/utils'
import { saveWebsocketTriggerFromCfg } from './websocket/utils'
import { savePostgresTriggerFromCfg } from './postgres/utils'
import { saveKafkaTriggerFromCfg } from './kafka/utils'
import { saveSqsTriggerFromCfg } from './sqs/utils'
import { saveNatsTriggerFromCfg } from './nats/utils'
import { saveMqttTriggerFromCfg } from './mqtt/utils'
import { saveGcpTriggerFromCfg } from './gcp/utils'

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
	| 'cli'

export type Trigger = {
	type: TriggerType
	path?: string
	isDraft?: boolean
	isPrimary?: boolean
	canWrite?: boolean
	id?: string
	draftConfig?: Record<string, any>
	captureConfig?: Record<string, any>
	extra?: Record<string, any>
	lightConfig?: Record<string, any>
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
	poll: SchedulePollIcon,
	cli: Terminal
}

export function isEqual(a: Trigger, b: Trigger): boolean {
	if (a.isDraft) {
		return a.id === b.id
	} else {
		return a.path === b.path && a.type === b.type
	}
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
	triggersCountStore: Writable<TriggersCount | undefined>,
	type: TriggerType,
	path?: string,
	draftCfg?: Record<string, any>
): Trigger {
	const currentTriggers = get(triggersStore)

	const primaryScheduleExists = currentTriggers.some((t) => t.type === 'schedule' && t.isPrimary)

	// Create the new draft trigger
	const draftId = generateDraftId()
	const isPrimary = type === 'schedule' && !primaryScheduleExists
	const newTrigger = {
		id: draftId,
		type,
		path,
		isPrimary,
		isDraft: true,
		draftConfig: draftCfg
	}

	// Add new draft to the store
	triggersStore.update((triggers) => [...triggers, newTrigger])

	updateTriggersCount(triggersCountStore, type, 'add', newTrigger.draftConfig)

	return newTrigger
}

function updateTriggersCount(
	triggersCountStore: Writable<TriggersCount | undefined>,
	type: TriggerType,
	action: 'add' | 'remove',
	primaryCfg?: Record<string, any>,
	isPrimary?: boolean
) {
	// Update the triggers count store
	if (type === 'schedule') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				if (primaryCfg) {
					return {
						...(triggersCount ?? {}),
						schedule_count: (triggersCount?.schedule_count ?? 0) + 1,
						primary_schedule: primaryCfg?.schedule
					}
				}
			} else {
				return {
					...(triggersCount ?? {}),
					schedule_count: (triggersCount?.schedule_count ?? 1) - 1,
					primary_schedule: isPrimary ? undefined : triggersCount?.primary_schedule
				}
			}
		})
	} else if (type === 'postgres') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					postgres_count: (triggersCount?.postgres_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					postgres_count: (triggersCount?.postgres_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'kafka') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					kafka_count: (triggersCount?.kafka_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					kafka_count: (triggersCount?.kafka_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'nats') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					nats_count: (triggersCount?.nats_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					nats_count: (triggersCount?.nats_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'mqtt') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					mqtt_count: (triggersCount?.mqtt_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					mqtt_count: (triggersCount?.mqtt_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'sqs') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					sqs_count: (triggersCount?.sqs_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					sqs_count: (triggersCount?.sqs_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'gcp') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					gcp_count: (triggersCount?.gcp_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					gcp_count: (triggersCount?.gcp_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'websocket') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					websocket_count: (triggersCount?.websocket_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					websocket_count: (triggersCount?.websocket_count ?? 1) - 1
				}
			}
		})
	} else if (type === 'http') {
		triggersCountStore.update((triggersCount) => {
			if (action === 'add') {
				return {
					...(triggersCount ?? {}),
					http_routes_count: (triggersCount?.http_routes_count ?? 0) + 1
				}
			} else {
				return {
					...(triggersCount ?? {}),
					http_routes_count: (triggersCount?.http_routes_count ?? 1) - 1
				}
			}
		})
	}
}

/**
 * Delete a draft trigger from the store
 */
export function deleteDraft(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	type: TriggerType,
	draftId: string,
	isPrimary?: boolean
): void {
	triggersStore.update((triggers) => triggers.filter((t) => t.id !== draftId))
	updateTriggersCount(triggersCountStore, type, 'remove', undefined, isPrimary)
}

/**
 * Delete a trigger from the store
 */
export function deleteTrigger(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	trigger: Trigger
): void {
	if (trigger.isDraft && trigger.id) {
		deleteDraft(triggersStore, triggersCountStore, trigger.type, trigger.id)
	} else {
		triggersStore.update((triggers) =>
			triggers.filter((t) => t.path !== trigger.path || t.type !== trigger.type)
		)
		updateTriggersCount(triggersCountStore, trigger.type, 'remove')
	}
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

/**
 * Fetch all types of triggers
 */
export async function fetchTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	primarySchedule: ScheduleTrigger | undefined | false = undefined,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return

	// Fetch each type of trigger
	await Promise.all([
		fetchSchedules(triggersStore, triggersCountStore, workspaceId, path, isFlow, primarySchedule),
		fetchHttpTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchWebsocketTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchPostgresTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchKafkaTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchNatsTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchMqttTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchSqsTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user),
		fetchGcpTriggers(triggersStore, triggersCountStore, workspaceId, path, isFlow, user)
	])
}

function updateTriggers(
	triggersStore: Writable<Trigger[]>,
	remoteTriggers: any[],
	type: TriggerType,
	user: UserExt | undefined = undefined
): number {
	const currentTriggers = get(triggersStore)
	// Identify triggers with draftConfig to preserve
	const configuredTriggers = currentTriggers.filter(
		(t) => t.type === type && !t.isDraft && t.draftConfig
	)

	const configMap = new Map<string, { draftConfig: Record<string, any> }>()

	configuredTriggers.forEach((t) => {
		configMap.set(t.path ?? '', { draftConfig: t.draftConfig! })
	})

	const backendTriggers = remoteTriggers.map((trigger) => {
		const { draftConfig } = configMap.get(trigger.path) ?? {}
		return {
			type: type as TriggerType,
			path: trigger.path,
			isPrimary: type === 'schedule' && trigger.path === trigger.script_path,
			isDraft: false,
			canWrite: canWrite(trigger.path, trigger.extra_perms, user),
			draftConfig: draftConfig,
			lightConfig:
				type === 'schedule' ? { schedule: trigger.schedule, enable: trigger.enable } : undefined
		}
	})

	const filteredTriggers = currentTriggers.filter((t) => t.type !== type || t.isDraft)
	const newTriggers = [...filteredTriggers, ...backendTriggers]
	triggersStore.set(newTriggers)

	return newTriggers.filter((t) => t.type === type).length
}

/**
 * Fetch schedule triggers
 */
export async function fetchSchedules(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	primarySchedule?: ScheduleTrigger | undefined | false,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		//First update the store with legacy primary schedule
		if (primarySchedule && !get(triggersStore).some((s) => s.isPrimary)) {
			const primary = {
				type: 'schedule' as TriggerType,
				path,
				isPrimary: true,
				isDraft: false,
				draftConfig: {
					schedule: primarySchedule.cron ? formatCron(primarySchedule.cron) : undefined,
					args: primarySchedule.args,
					timezone: primarySchedule.timezone,
					summary: primarySchedule.summary,
					description: primarySchedule.description,
					enabled: primarySchedule.enabled
				}
			}
			triggersStore.update((triggers) => [...triggers, primary])
		}

		const allDeployedSchedules: Schedule[] = await ScheduleService.listSchedules({
			workspace: workspaceId,
			path,
			isFlow
		})

		const scheduleCount = updateTriggers(triggersStore, allDeployedSchedules, 'schedule', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				schedule_count: scheduleCount
			}
		})

		return
	} catch (error) {
		console.error('Failed to fetch schedules:', error)
		return
	}
}

/**
 * Fetch HTTP triggers
 */
export async function fetchHttpTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
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
		const httpCount = updateTriggers(triggersStore, httpTriggers, 'http', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				http_routes_count: httpCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch HTTP triggers:', error)
	}
}

/**
 * Fetch Websocket triggers
 */
export async function fetchWebsocketTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
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
		const wsCount = updateTriggers(triggersStore, wsTriggers, 'websocket', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				websocket_count: wsCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch Websocket triggers:', error)
	}
}

/**
 * Fetch Postgres triggers
 */
export async function fetchPostgresTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
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
		const pgCount = updateTriggers(triggersStore, pgTriggers, 'postgres', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				postgres_count: pgCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch Postgres triggers:', error)
	}
}

/**
 * Fetch Kafka triggers
 */
export async function fetchKafkaTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
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
		const kafkaCount = updateTriggers(triggersStore, kafkaTriggers, 'kafka', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				kafka_count: kafkaCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch Kafka triggers:', error)
	}
}

/**
 * Fetch NATS triggers
 */
export async function fetchNatsTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const natsTriggers = await NatsTriggerService.listNatsTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		const natsCount = updateTriggers(triggersStore, natsTriggers, 'nats', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				nats_count: natsCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch NATS triggers:', error)
	}
}

/**
 * Fetch MQTT triggers
 */
export async function fetchMqttTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const mqttTriggers = await MqttTriggerService.listMqttTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		const mqttCount = updateTriggers(triggersStore, mqttTriggers, 'mqtt', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				mqtt_count: mqttCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch MQTT triggers:', error)
	}
}

/**
 * Fetch SQS triggers
 */
export async function fetchSqsTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	workspaceId: string | undefined,
	path: string,
	isFlow: boolean,
	user: UserExt | undefined = undefined
): Promise<void> {
	if (!workspaceId) return
	try {
		const sqsTriggers = await SqsTriggerService.listSqsTriggers({
			workspace: workspaceId,
			path,
			isFlow
		})
		const sqsCount = updateTriggers(triggersStore, sqsTriggers, 'sqs', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				sqs_count: sqsCount
			}
		})
	} catch (error) {
		console.error('Failed to fetch SQS triggers:', error)
	}
}

/**
 * Fetch GCP Pub/Sub triggers
 */
export async function fetchGcpTriggers(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
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
		const gcpCount = updateTriggers(triggersStore, gcpTriggers, 'gcp', user)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				gcp_count: gcpCount
			}
		})
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

export async function deployTriggers(
	triggersToDeploy: Trigger[],
	workspaceId: string | undefined,
	isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>,
	initialPath?: string
) {
	if (!workspaceId) return
	await Promise.all(
		triggersToDeploy.map((t) => {
			if (t.type === 'schedule') {
				if (t.isPrimary && initialPath) {
					t.draftConfig = {
						...t.draftConfig,
						path: initialPath,
						script_path: initialPath
					}
				}
				saveScheduleFromCfg(t.draftConfig ?? {}, !t.isDraft, workspaceId)
			} else if (t.type === 'http') {
				saveHttpRouteFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					isAdmin,
					usedTriggerKinds
				)
			} else if (t.type === 'websocket') {
				saveWebsocketTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'postgres') {
				savePostgresTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'kafka') {
				saveKafkaTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'sqs') {
				saveSqsTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'nats') {
				saveNatsTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'mqtt') {
				saveMqttTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			} else if (t.type === 'gcp') {
				saveGcpTriggerFromCfg(
					t.path ?? t.draftConfig?.path ?? '',
					t.draftConfig ?? {},
					!t.isDraft,
					workspaceId,
					usedTriggerKinds
				)
			}
		})
	)
}

export function handleSelectTriggerFromKind(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	selectedTriggerStore: Writable<Trigger | undefined>,
	initialPath: string | undefined,
	triggerKind: TriggerKind
) {
	const triggerType = triggerKindToTriggerType(triggerKind)

	if (!triggerType) {
		return
	}

	const existingTrigger = get(triggersStore).find((trigger) => trigger.type === triggerType)

	if (existingTrigger) {
		selectedTriggerStore.set(existingTrigger)
	} else {
		const newTrigger = addDraftTrigger(
			triggersStore,
			triggersCountStore,
			triggerType,
			triggerType === 'schedule' ? initialPath : undefined
		)
		selectedTriggerStore.set(newTrigger)
	}
}
