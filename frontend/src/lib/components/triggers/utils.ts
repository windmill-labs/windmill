import { Webhook, Mail, Calendar, Route, Unplug, Database, Terminal } from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
import type {
	CaptureTriggerKind,
	ErrorHandler,
	Flow,
	JobTriggerKind,
	NewScript,
	TriggersCount
} from '$lib/gen/types.gen'
import type { Writable } from 'svelte/store'
import SchedulePollIcon from '../icons/SchedulePollIcon.svelte'
import type { TriggerKind } from '../triggers'
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
import { emptyString } from '$lib/utils'
import { saveEmailTriggerFromCfg } from './email/utils'
import NextcloudIcon from '$lib/components/icons/NextcloudIcon.svelte'
import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'
import { saveNativeTriggerFromCfg } from './native/utils'

export const CLOUD_DISABLED_TRIGGER_TYPES = [
	'nats',
	'kafka',
	'sqs',
	'mqtt',
	'gcp',
	'websocket',
	'postgres'
]

export type TriggerType =
	| 'webhook'
	| 'default_email'
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
	| 'email'
	| 'poll'
	| 'cli'
	| 'nextcloud'
	| 'google'

export const jobTriggerKinds: JobTriggerKind[] = [
	'webhook',
	'default_email',
	'http',
	'websocket',
	'kafka',
	'email',
	'nats',
	'mqtt',
	'sqs',
	'postgres',
	'schedule',
	'gcp',
	'google'
]

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
	default_email: Mail,
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
	cli: Terminal,
	nextcloud: NextcloudIcon,
	google: GoogleIcon
}

export const triggerDisplayNamesMap = {
	schedule: 'Schedule',
	http: 'HTTP',
	websocket: 'WebSocket',
	postgres: 'Postgres',
	kafka: 'Kafka',
	nats: 'NATS',
	mqtt: 'MQTT',
	sqs: 'SQS',
	gcp: 'GCP Pub/Sub',
	email: 'Email',
	poll: 'Scheduled Poll',
	webhook: 'Webhook',
	default_email: 'Default Email',
	cli: 'CLI',
	nextcloud: 'Nextcloud',
	google: 'Google'
} as const satisfies Record<TriggerType, string>

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
		'default_email',
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
		default_email: undefined,
		schedule: 'schedule_count',
		http: 'http_routes_count',
		websocket: 'websocket_count',
		postgres: 'postgres_count',
		kafka: 'kafka_count',
		nats: 'nats_count',
		mqtt: 'mqtt_count',
		sqs: 'sqs_count',
		gcp: 'gcp_count',
		email: 'email_count',
		poll: undefined,
		cli: undefined,
		nextcloud: 'nextcloud_count',
		google: 'google_count'
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
		case 'default_emails':
			return 'default_email'
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
		default_email: undefined,
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
		email: (trigger: Trigger) =>
			saveEmailTriggerFromCfg(
				trigger.path ?? trigger.draftConfig?.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				isAdmin,
				usedTriggerKinds
			),
		poll: undefined,
		cli: undefined,
		nextcloud: (trigger: Trigger) =>
			saveNativeTriggerFromCfg(
				'nextcloud',
				trigger.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			),
		google: (trigger: Trigger) =>
			saveNativeTriggerFromCfg(
				'google',
				trigger.path ?? '',
				trigger.draftConfig ?? {},
				!trigger.isDraft,
				workspaceId,
				usedTriggerKinds
			)
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

export function getLightConfig(
	triggerType: TriggerType,
	trigger: Record<string, any>
): Record<string, any> | undefined {
	if (triggerType === 'schedule') {
		return { schedule: trigger.schedule, enable: trigger.enable, summary: trigger.summary }
	} else if (triggerType === 'http') {
		return { route_path: trigger.route_path, http_method: trigger.http_method }
	} else if (triggerType === 'websocket') {
		return { url: trigger.url }
	} else if (triggerType === 'postgres') {
		return { postgres_resource_path: trigger.postgres_resource_path }
	} else if (triggerType === 'kafka') {
		return { kafka_resource_path: trigger.kafka_resource_path, topics: trigger.topics }
	} else if (triggerType === 'nats') {
		return { nats_resource_path: trigger.nats_resource_path, subjects: trigger.subjects }
	} else if (triggerType === 'mqtt') {
		return {
			mqtt_resource_path: trigger.mqtt_resource_path,
			subscribe_topics: trigger.subscribe_topics
		}
	} else if (triggerType === 'sqs') {
		return { queue_url: trigger.queue_url }
	} else if (triggerType === 'gcp') {
		return { gcp_resource_path: trigger.gcp_resource_path, topic: trigger.topic }
	} else if (triggerType === 'email') {
		return { local_part: trigger.local_part }
	} else if (triggerType === 'nextcloud') {
		return { event: trigger.service_config?.event ?? trigger.event }
	} else if (triggerType === 'google') {
		return {
			trigger_type: trigger.service_config?.triggerType ?? trigger.trigger_type,
			resource_id: trigger.service_config?.resourceId ?? trigger.resource_id,
			resource_name: trigger.service_config?.resourceName ?? trigger.resource_name,
			calendar_id: trigger.service_config?.calendarId ?? trigger.calendar_id,
			calendar_name: trigger.service_config?.calendarName ?? trigger.calendar_name
		}
	} else {
		return undefined
	}
}

export function getTriggerLabel(trigger: Trigger): string {
	const { type, isDraft, draftConfig, lightConfig, path } = trigger
	const config = draftConfig ?? lightConfig

	if (type === 'webhook') {
		return 'Webhook'
	} else if (type === 'default_email') {
		return 'Email'
	} else if (type === 'cli') {
		return 'CLI'
	} else if (type === 'http' && !emptyString(config?.route_path)) {
		return `${(draftConfig?.http_method ?? lightConfig?.http_method ?? 'post').toUpperCase()} ${draftConfig?.route_path ?? lightConfig?.route_path}`
	} else if (type === 'schedule' && config?.summary) {
		return `${config?.summary}`
	} else if (type === 'kafka' && config?.topics && config?.kafka_resource_path) {
		return `${config?.kafka_resource_path} - ${config?.topics.join(', ')}`
	} else if (type === 'nats' && config?.subjects && config?.nats_resource_path) {
		return `${config?.nats_resource_path} - ${config?.subjects.join(', ')}`
	} else if (type === 'mqtt' && config?.subscribe_topics && config?.mqtt_resource_path) {
		const topics = config?.subscribe_topics.map((topic: any) => topic.topic).join(', ')
		return `${config?.mqtt_resource_path} - ${topics}`
	} else if (type === 'sqs' && config?.queue_url) {
		return `${config?.queue_url}`
	} else if (type === 'gcp' && config?.gcp_resource_path && config?.topic) {
		return `${config?.gcp_resource_path} - ${config?.topic}`
	} else if (type === 'websocket' && config?.url) {
		return `${config?.url}`
	} else if (type === 'email' && config?.local_part) {
		return `${config?.local_part}`
	} else if (type === 'nextcloud' && path) {
		return `${path}`
	} else if (type === 'google' && path) {
		const triggerType = config?.trigger_type ?? config?.triggerType
		if (triggerType === 'calendar') {
			const name = config?.resource_name ?? config?.calendar_id ?? ''
			return `Calendar: ${name || path}`
		} else {
			const name = config?.resource_name ?? ''
			return name ? `Drive: ${name}` : config?.resource_id ? `Drive: ${path}` : `Drive: All changes`
		}
	} else if (isDraft && draftConfig?.path) {
		return `${draftConfig?.path}`
	} else if (isDraft) {
		return `New ${type.replace(/s$/, '')} trigger`
	} else {
		return path ?? ''
	}
}

export function sortTriggers(triggers: Trigger[]): Trigger[] {
	const triggerTypeOrder = [
		'webhook',
		'cli',
		'default_email',
		'poll',
		'schedule',
		'http',
		'websocket',
		'postgres',
		'kafka',
		'nats',
		'mqtt',
		'sqs',
		'gcp',
		'email',
		'nextcloud',
		'google'
	]

	return triggers.sort((a, b) => {
		// Draft triggers always come last
		if (a.isDraft && !b.isDraft) return 1
		if (!a.isDraft && b.isDraft) return -1

		// If both are drafts or both are not drafts, sort by type order
		if (a.isDraft === b.isDraft) {
			const aIndex = triggerTypeOrder.indexOf(a.type)
			const bIndex = triggerTypeOrder.indexOf(b.type)

			// If both types are in the order array, sort by their position
			if (aIndex >= 0 && bIndex >= 0) {
				return aIndex - bIndex
			}

			// If only one type is in the order array, it comes first
			if (aIndex >= 0) return -1
			if (bIndex >= 0) return 1

			// If neither type is in the order array, maintain original order
			return 0
		}

		return 0
	})
}

export type FlowWithDraftAndDraftTriggers = Flow & {
	draft?: Flow & {
		draft_triggers?: Trigger[]
	}
}

export type NewScriptWithDraftAndDraftTriggers = NewScript & {
	draft?: NewScript & { draft_triggers?: Trigger[] }
	hash: string
}

// Get rid of deployed triggers from the saved flow in the case there is a match with a deployed trigger
export function filterDraftTriggers(
	savedValue: FlowWithDraftAndDraftTriggers | NewScriptWithDraftAndDraftTriggers,
	triggersState: Triggers
): FlowWithDraftAndDraftTriggers | NewScriptWithDraftAndDraftTriggers {
	const deployedTriggers = triggersState.triggers.filter((t) => !t.draftConfig && !t.isDraft)

	// Early return if no deployed triggers or no draft triggers to filter
	if (deployedTriggers.length === 0 || !savedValue?.draft?.draft_triggers?.length) {
		return savedValue
	}

	const deployedTriggerKeys = new Set(deployedTriggers.map((t) => `${t.path}:${t.type}`))

	const originalSavedDraftTriggers = savedValue.draft.draft_triggers
	const keptTriggers: Trigger[] = []
	const removedTriggers: Trigger[] = []

	// Single pass to separate kept vs removed triggers
	for (const savedTrigger of originalSavedDraftTriggers) {
		const triggerKey = `${savedTrigger.draftConfig?.path}:${savedTrigger.type}`
		if (deployedTriggerKeys.has(triggerKey)) {
			removedTriggers.push(savedTrigger)
		} else {
			keptTriggers.push(savedTrigger)
		}
	}

	// Early return if nothing was filtered
	if (removedTriggers.length === 0) {
		return savedValue
	}

	// Update saved value
	const newSavedValue = {
		...savedValue,
		draft: {
			...savedValue.draft,
			draft_triggers: keptTriggers.length > 0 ? keptTriggers : undefined
		}
	} as typeof savedValue

	const removedTriggerKeys = new Set(removedTriggers.map((t) => `${t.draftConfig?.path}:${t.type}`))

	// Remove filtered triggers from triggersState
	triggersState.setTriggers(
		triggersState.triggers.filter((trigger) => {
			const triggerKey = `${trigger.draftConfig?.path}:${trigger.type}`
			return !removedTriggerKeys.has(triggerKey)
		})
	)

	return newSavedValue
}

export function getHandlerType(scriptPath: string): ErrorHandler {
	const handlerMap = {
		teams: '/workspace-or-schedule-error-handler-teams',
		slack: '/workspace-or-schedule-error-handler-slack',
		email: '/workspace-or-error-handler-email'
	}
	for (const [type, suffix] of Object.entries(handlerMap)) {
		if (scriptPath.startsWith('hub/') && scriptPath.endsWith(suffix)) {
			return type as ErrorHandler
		}
	}
	return 'custom'
}
