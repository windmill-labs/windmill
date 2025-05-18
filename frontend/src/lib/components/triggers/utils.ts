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

/**
 * Converts a TriggerType to a CaptureTriggerKind when a mapping exists
 * @param triggerType The trigger type to convert
 * @returns The corresponding CaptureTriggerKind or undefined if no mapping exists
 */
export function triggerTypeToCaptureKind(triggerType: TriggerType): CaptureTriggerKind | undefined {
	// Define types that can be mapped to CaptureTriggerKind
	const capturableTriggerTypes: TriggerType[] = [
		'webhook',
		'email',
		'http',
		'websocket',
		'postgres',
		'kafka',
		'nats',
		'mqtt',
		'sqs',
		'gcp',
		'cli'
	]

	if (capturableTriggerTypes.includes(triggerType)) {
		return triggerType as CaptureTriggerKind
	}

	return undefined
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
): number {
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

	return currentTriggers.length
}

function updateTriggersCount(
	triggersCountStore: Writable<TriggersCount | undefined>,
	type: TriggerType,
	action: 'add' | 'remove',
	primaryCfg?: Record<string, any>,
	isPrimary?: boolean
) {
	// Map trigger types to their corresponding count property names
	const countPropertyMap: Record<TriggerType, string | undefined> = {
		webhook: undefined,
		email: undefined,
		schedule: 'schedule_count',
		http: 'http_routes_count',
		websocket: 'websocket_count',
		postgres: 'postgres_count',
		kafka: 'kafka_count',
		nats: 'nats_count',
		mqtt: 'mqtt_count',
		sqs: 'sqs_count',
		gcp: 'gcp_count',
		poll: undefined,
		cli: undefined
	}

	const countProperty = countPropertyMap[type]

	triggersCountStore.update((triggersCount) => {
		// Handle special case for schedule
		if (type === 'schedule') {
			if (action === 'add' && primaryCfg) {
				return {
					...(triggersCount ?? {}),
					schedule_count: (triggersCount?.schedule_count ?? 0) + 1,
					primary_schedule: primaryCfg?.schedule
				}
			} else if (action === 'remove') {
				return {
					...(triggersCount ?? {}),
					schedule_count: (triggersCount?.schedule_count ?? 1) - 1,
					primary_schedule: isPrimary ? undefined : triggersCount?.primary_schedule
				}
			}
		}

		// Handle standard count updates
		if (countProperty && action === 'add') {
			return {
				...(triggersCount ?? {}),
				[countProperty]: (triggersCount?.[countProperty] ?? 0) + 1
			}
		} else if (countProperty && action === 'remove') {
			return {
				...(triggersCount ?? {}),
				[countProperty]: (triggersCount?.[countProperty] ?? 1) - 1
			}
		}

		return triggersCount
	})
}

/**
 * Delete a trigger from the store
 */
export function deleteTrigger(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	triggerIndex: number
): void {
	const { type, isDraft } = get(triggersStore)[triggerIndex]

	triggersStore.update((triggers) => {
		return triggers.filter((_, index) => index !== triggerIndex)
	})

	if (!isDraft) {
		updateTriggersCount(triggersCountStore, type, 'remove')
	}
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
		fetchSchedules(
			triggersStore,
			triggersCountStore,
			workspaceId,
			path,
			isFlow,
			primarySchedule,
			user
		),
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
		const updatedPrimarySchedule = get(triggersStore).find((s) => s.isPrimary)
		triggersCountStore.update((triggersCount) => {
			return {
				...(triggersCount ?? {}),
				schedule_count: scheduleCount,
				primary_schedule: {
					schedule:
						updatedPrimarySchedule?.draftConfig?.schedule ??
						updatedPrimarySchedule?.lightConfig?.schedule
				}
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

	// Map of trigger types to their save functions
	const triggerSaveFunctions: Record<TriggerType, Function | undefined> = {
		webhook: undefined,
		email: undefined,
		schedule: (trigger: Trigger) => {
			if (trigger.isPrimary && initialPath) {
				trigger.draftConfig = {
					...trigger.draftConfig,
					path: initialPath,
					script_path: initialPath
				}
			}
			return saveScheduleFromCfg(trigger.draftConfig ?? {}, !trigger.isDraft, workspaceId)
		},
		http: (trigger: Trigger) =>
			saveHttpRouteFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				isAdmin,
				usedTriggerKinds
			),
		websocket: (trigger: Trigger) =>
			saveWebsocketTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		postgres: (trigger: Trigger) =>
			savePostgresTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		kafka: (trigger: Trigger) =>
			saveKafkaTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		nats: (trigger: Trigger) =>
			saveNatsTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		mqtt: (trigger: Trigger) =>
			saveMqttTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		sqs: (trigger: Trigger) =>
			saveSqsTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		gcp: (trigger: Trigger) =>
			saveGcpTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		poll: undefined,
		cli: undefined
	}

	await Promise.all(
		triggersToDeploy.map(async (trigger) => {
			const saveFunction = triggerSaveFunctions[trigger.type]
			if (saveFunction) {
				await saveFunction(trigger)
			} else {
				console.warn(`No save function defined for trigger type: ${trigger.type}`)
			}
		})
	)
}

export function handleSelectTriggerFromKind(
	triggersStore: Writable<Trigger[]>,
	triggersCountStore: Writable<TriggersCount | undefined>,
	selectedTriggerStore: Writable<number | undefined>,
	initialPath: string | undefined,
	triggerKind: TriggerKind
) {
	const triggerType = triggerKindToTriggerType(triggerKind)

	if (!triggerType) {
		return
	}

	const existingTriggerIndex = get(triggersStore).findIndex(
		(trigger) => trigger.type === triggerType
	)

	if (existingTriggerIndex !== -1) {
		selectedTriggerStore.set(existingTriggerIndex)
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

export function handleConfigChange(
	nCfg: Record<string, any>,
	initialConfig: Record<string, any> | undefined,
	saveDisabled: boolean,
	edit: boolean,
	onConfigChange?: (cfg: Record<string, any>, saveDisabled: boolean, updated: boolean) => void
) {
	let updated = false
	if (!edit || !initialConfig) {
		updated = true
	} else {
		// We ignore changes to enabled
		let newCfg = { ...nCfg }
		if ('enabled' in newCfg) {
			delete newCfg.enabled
		}
		let initialCfg = { ...initialConfig }
		if ('enabled' in initialCfg) {
			delete initialCfg.enabled
		}
		if (JSON.stringify(newCfg) !== JSON.stringify(initialCfg)) {
			updated = true
		}
	}

	onConfigChange?.(nCfg, saveDisabled, updated)
}
