import { Webhook, Mail, Calendar, Route, Unplug, Database, Terminal } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
import type { CaptureTriggerKind, TriggersCount } from '$lib/gen/types.gen'
import type { Writable } from 'svelte/store'
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
import type { Triggers } from './triggers.svelte'

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

export function updateTriggersCount(
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
	initialPath?: string,
	isNew?: boolean
) {
	if (!workspaceId) return

	if (isNew && initialPath) {
		triggersToDeploy.forEach((trigger) => {
			trigger.draftConfig = {
				...trigger.draftConfig,
				script_path: initialPath
			}
		})
	}

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
	triggersState: Triggers,
	triggersCountStore: Writable<TriggersCount | undefined>,
	initialPath: string | undefined,
	triggerKind: TriggerKind
) {
	const triggerType = triggerKindToTriggerType(triggerKind)

	if (!triggerType) {
		return
	}

	const existingTriggerIndex = triggersState.triggers.findIndex(
		(trigger) => trigger.type === triggerType
	)

	if (existingTriggerIndex !== -1) {
		triggersState.selectedTriggerIndex = existingTriggerIndex
	} else {
		const newTrigger = triggersState.addDraftTrigger(
			triggersCountStore,
			triggerType,
			triggerType === 'schedule' ? initialPath : undefined
		)
		triggersState.selectedTriggerIndex = newTrigger
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
