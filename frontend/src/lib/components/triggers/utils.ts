import {
	Webhook,
	Mail,
	Calendar,
	Route,
	Unplug,
	Database,
	Terminal,
	Timer,
	Zap,
	LayoutDashboard
} from 'lucide-svelte'
import KafkaIcon from '$lib/components/icons/KafkaIcon.svelte'
import NatsIcon from '$lib/components/icons/NatsIcon.svelte'
import MqttIcon from '$lib/components/icons/MqttIcon.svelte'
import AmqpIcon from '$lib/components/icons/AmqpIcon.svelte'
import AwsIcon from '$lib/components/icons/AwsIcon.svelte'
import GoogleCloudIcon from '$lib/components/icons/GoogleCloudIcon.svelte'
import AzureIcon from '$lib/components/icons/AzureIcon.svelte'
import type {
	CaptureTriggerKind,
	ErrorHandler,
	Flow,
	JobTriggerKind,
	NewScript,
	TriggersCount
} from '$lib/gen/types.gen'
import { get, type Writable } from 'svelte/store'
import type { UserDraftItemKind } from '$lib/userDraft.svelte'
import { userStore } from '$lib/stores'
import SchedulePollIcon from '../icons/SchedulePollIcon.svelte'
import type { TriggerKind } from '../triggers'
import type { Triggers } from './triggers.svelte'
import { emptyString, generateRandomString } from '$lib/utils'
import NextcloudIcon from '$lib/components/icons/NextcloudIcon.svelte'
import GoogleIcon from '$lib/components/icons/GoogleIcon.svelte'
import GithubIcon from '$lib/components/icons/GithubIcon.svelte'
import { saveNativeTriggerFromCfg } from './native/utils'
import { DraftService, type NativeServiceName } from '$lib/gen'
import { UserDraft } from '$lib/userDraft.svelte'
import { deployDraft } from '$lib/utils_draft_deploy'

export const CLOUD_DISABLED_TRIGGER_TYPES = [
	'nats',
	'kafka',
	'sqs',
	'mqtt',
	'amqp',
	'gcp',
	'azure',
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
	| 'amqp'
	| 'sqs'
	| 'gcp'
	| 'azure'
	| 'email'
	| 'poll'
	| 'cli'
	| 'nextcloud'
	| 'google'
	| 'github'

export const jobTriggerKinds: JobTriggerKind[] = [
	'webhook',
	'default_email',
	'http',
	'websocket',
	'kafka',
	'email',
	'nats',
	'mqtt',
	'amqp',
	'sqs',
	'postgres',
	'schedule',
	'gcp',
	'azure',
	'google',
	'github',
	'asset',
	'freshness',
	'app'
]

export type Trigger = {
	type: TriggerType
	path?: string
	/** No deployed row at this path — the trigger exists only as a draft. */
	isDraft?: boolean
	isPrimary?: boolean
	canWrite?: boolean
	id?: string
	/** This user has a `trigger_*` draft row at `path`. On a deployed trigger it
	 * means "modified"; on a draft-only one it is implied by `isDraft`. */
	hasDraft?: boolean
	/** Where a draft-only trigger will deploy to, when renamed away from `path`
	 * (which stays the draft row's key). Display this, address by `path`. */
	draftPath?: string
	/** Added in this editor session and not yet written to the `draft` table. Its
	 * editor opens through `openNew`, the path that fills a complete config; the
	 * resulting autosave is what creates the row. */
	isNew?: boolean
	/** Editor-local unsaved config, for the trigger kinds that have no draft row
	 * yet (native only — see `triggerDraftKind`). */
	draftConfig?: Record<string, any>
	captureConfig?: Record<string, any>
	extra?: Record<string, any>
	lightConfig?: Record<string, any>
}

/** The runnable a trigger drafted in an editor fires, resolved live: the flow or
 * script path can still be renamed before it is deployed. */
export type TriggerDraftTarget = {
	runnablePath: string
	isFlow: boolean
	workspace: string | undefined
}

/** For the read-only `TriggerContext` mounts (details pages, webhook viewer):
 * they render triggers but have no editor, so nothing can be drafted. The
 * absent workspace makes `addDraftTrigger` skip the draft row rather than
 * write one against a runnable it doesn't know. */
export const NO_TRIGGER_DRAFT_TARGET: TriggerDraftTarget = {
	runnablePath: '',
	isFlow: false,
	workspace: undefined
}

/** The `draft` table kind backing each trigger type's per-user drafts.
 * `undefined` means the type has no draft row: the pseudo-triggers (webhook,
 * default_email, cli, poll) have no config to draft, and the native kinds
 * (nextcloud/google/github) are keyed by a service-assigned `external_id` with
 * permissions on the runnable, which the path-keyed draft table can't express —
 * they stay on the editor-local `draftConfig` until that is wired. */
const TRIGGER_DRAFT_KIND = {
	webhook: undefined,
	default_email: undefined,
	poll: undefined,
	cli: undefined,
	nextcloud: undefined,
	google: undefined,
	github: undefined,
	schedule: 'trigger_schedule',
	http: 'trigger_http',
	websocket: 'trigger_websocket',
	postgres: 'trigger_postgres',
	kafka: 'trigger_kafka',
	nats: 'trigger_nats',
	mqtt: 'trigger_mqtt',
	amqp: 'trigger_amqp',
	sqs: 'trigger_sqs',
	gcp: 'trigger_gcp',
	azure: 'trigger_azure',
	email: 'trigger_email'
} as const satisfies Record<TriggerType, UserDraftItemKind | undefined>

export function triggerDraftKind(type: TriggerType): UserDraftItemKind | undefined {
	return TRIGGER_DRAFT_KIND[type]
}

/** Whether the trigger diverges from what is deployed — a draft row for the
 * draft-backed kinds, an editor-local config for the native ones. Drives the
 * "Modified" badge, the reset affordance and the deploy-time confirmation. */
export function triggerHasPendingChanges(trigger: Trigger): boolean {
	return !!trigger.hasDraft || !!trigger.draftConfig
}

/** Storage path for a trigger drafted from the flow/script editor. Draft rows
 * are path-keyed, so one is needed the moment the trigger is added — before the
 * user has typed anything. A schedule that will be the primary takes the
 * runnable's own path (what the deployed primary schedule uses); everything else
 * is suffixed by kind, then disambiguated so two triggers of a kind don't
 * collide. `taken` is every path already in use for this kind. */
export function newDraftTriggerPath(
	runnablePath: string,
	type: TriggerType,
	taken: string[],
	isPrimarySchedule = false
): string {
	if (type === 'schedule' && isPrimarySchedule && runnablePath && !taken.includes(runnablePath)) {
		return runnablePath
	}
	const base = `${runnablePath || `u/${get(userStore)?.username ?? 'user'}/trigger`}_${type}`
	if (!taken.includes(base)) return base
	let candidate = base
	while (taken.includes(candidate)) {
		candidate = `${base}_${generateRandomString(4)}`
	}
	return candidate
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
	amqp: AmqpIcon,
	sqs: AwsIcon,
	gcp: GoogleCloudIcon,
	azure: AzureIcon,
	primary_schedule: Calendar,
	poll: SchedulePollIcon,
	cli: Terminal,
	nextcloud: NextcloudIcon,
	google: GoogleIcon,
	github: GithubIcon,
	// Job-attribution-only kinds (no trigger CRUD page): the pipeline asset
	// cascade, the freshness watchdog, and app-component runs. Needed so the Runs
	// filter and job detail render these trigger kinds instead of a blank label /
	// no icon.
	asset: Zap,
	freshness: Timer,
	app: LayoutDashboard
}

export const triggerDisplayNamesMap = {
	schedule: 'Schedule',
	http: 'HTTP',
	websocket: 'WebSocket',
	postgres: 'Postgres',
	kafka: 'Kafka',
	nats: 'NATS',
	mqtt: 'MQTT',
	amqp: 'AMQP',
	sqs: 'SQS',
	gcp: 'GCP Pub/Sub',
	azure: 'Azure Event Grid',
	email: 'Email',
	poll: 'Scheduled Poll',
	webhook: 'Webhook',
	default_email: 'Default Email',
	cli: 'CLI',
	nextcloud: 'Nextcloud',
	google: 'Google',
	github: 'GitHub',
	asset: 'Asset cascade',
	freshness: 'Freshness',
	app: 'App'
	// `asset` / `freshness` / `app` are job-attribution-only (JobTriggerKind, not
	// TriggerType) — hence the union in the satisfies below.
} as const satisfies Record<TriggerType | 'asset' | 'freshness' | 'app', string>

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
		'amqp',
		'sqs',
		'gcp',
		'azure',
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
		amqp: 'amqp_count',
		sqs: 'sqs_count',
		gcp: 'gcp_count',
		azure: 'azure_count',
		email: 'email_count',
		poll: undefined,
		cli: undefined,
		nextcloud: 'nextcloud_count',
		google: 'google_count',
		github: 'github_count'
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
		case 'amqp':
			return 'amqp'
		case 'sqs':
			return 'sqs'
		case 'gcp':
			return 'gcp'
		case 'azure':
			return 'azure'
		case 'scheduledPoll':
			return 'poll'
		default:
			throw new Error(`Unknown TriggerKind: ${kind}`)
	}
}

/** The stored draft config for a trigger. Prefers the live in-memory cell — an
 * open editor's not-yet-flushed keystrokes — and falls back to the server row,
 * the only source for a trigger whose editor panel was never mounted this
 * session. `undefined` when there is no draft at all. */
async function readTriggerDraft(
	kind: UserDraftItemKind,
	path: string,
	workspace: string
): Promise<Record<string, any> | undefined> {
	const local = UserDraft.get<Record<string, any>>(kind, path, { workspace })
	if (local !== undefined) return local
	try {
		// `getOwnDraft` returns the draft ROW (`{ value, created_at }`), not the
		// config — writing the wrapper back would nest the draft inside itself.
		const row = (await DraftService.getOwnDraft({ workspace, kind, path })) as
			| { value?: Record<string, any> }
			| undefined
		return row?.value ?? undefined
	} catch (err) {
		console.error(`Could not read ${kind} draft at ${path}`, err)
		return undefined
	}
}

/** Repoint a runnable's undeployed trigger drafts at its current path. The path
 * stays editable until the runnable is first deployed, and a trigger drafted
 * before a rename would otherwise fire the old path — including when it is
 * deployed on its own from the triggers page or Review & Deploy. Only rewrites
 * drafts that have actually fallen behind. */
export async function repointTriggerDrafts(
	triggers: Trigger[],
	target: TriggerDraftTarget
): Promise<void> {
	const { runnablePath, isFlow, workspace } = target
	if (!workspace || !runnablePath) return
	await Promise.all(
		triggers.map(async (trigger) => {
			const kind = triggerDraftKind(trigger.type)
			if (!kind || !trigger.path || !triggerHasPendingChanges(trigger)) return
			const draft = await readTriggerDraft(kind, trigger.path, workspace)
			if (!draft) return
			if (draft.script_path === runnablePath && draft.is_flow === isFlow) return
			UserDraft.save(
				kind,
				trigger.path,
				{ ...draft, script_path: runnablePath, is_flow: isFlow },
				{ workspace }
			)
		})
	)
}

/** Deploy the pending changes of `triggersToDeploy` as part of deploying their
 * runnable. The draft-backed kinds replay their stored draft through the same
 * `deployDraft` the drafts page uses, after re-pointing it at `runnablePath` —
 * a never-deployed runnable's path is still editable, and the primary schedule
 * additionally takes the runnable's path as its own. The native kinds have no
 * draft row and deploy from their editor-local config.
 *
 * Returns the triggers that failed, so the caller can report them without
 * aborting the rest of the deploy. */
export async function deployTriggers(
	triggersToDeploy: Trigger[],
	workspaceId: string | undefined,
	isAdmin: boolean,
	usedTriggerKinds: Writable<string[]>,
	runnablePath: string,
	isFlow: boolean
): Promise<{ trigger: Trigger; error: string }[]> {
	if (!workspaceId) return []

	const nativeSavers: Partial<Record<TriggerType, NativeServiceName>> = {
		nextcloud: 'nextcloud',
		google: 'google',
		github: 'github'
	}

	const results = await Promise.all(
		triggersToDeploy.map(async (trigger) => {
			const draftKind = triggerDraftKind(trigger.type)
			if (draftKind && trigger.path) {
				// Re-point the draft at the path the runnable is deploying to. The
				// primary schedule is identified by sharing that path, so it moves too.
				const draft = await readTriggerDraft(draftKind, trigger.path, workspaceId)
				if (!draft) {
					return { trigger, error: 'Draft not found' }
				}
				const repointed = {
					...draft,
					script_path: runnablePath,
					is_flow: isFlow,
					...(trigger.isPrimary ? { path: runnablePath } : {})
				}
				UserDraft.save(draftKind, trigger.path, repointed, { workspace: workspaceId })
				await UserDraft.forcePersist(draftKind, trigger.path, { workspace: workspaceId })
				const res = await deployDraft(draftKind, trigger.path, workspaceId, {
					draftOnly: trigger.isDraft
				})
				if (!res.success) {
					return { trigger, error: res.error ?? 'Deploy failed' }
				}
				// `deployDraft` deleted the row, but an editor panel open on this
				// trigger still holds the config in its cell and would autosave it
				// straight back as a fresh draft. Reset the cell to what was just
				// deployed so the persist-effect sees no divergence.
				UserDraft.discard(draftKind, trigger.path, repointed, { workspace: workspaceId })
				return undefined
			}

			const service = nativeSavers[trigger.type]
			if (service) {
				const ok = await saveNativeTriggerFromCfg(
					service,
					trigger.path ?? '',
					{ ...trigger.draftConfig, script_path: runnablePath, is_flow: isFlow },
					!trigger.isDraft,
					workspaceId,
					usedTriggerKinds
				)
				return ok ? undefined : { trigger, error: `Could not deploy ${trigger.type} trigger` }
			}

			return undefined
		})
	)
	return results.filter((r) => r !== undefined)
}

export async function handleSelectTriggerFromKind(
	triggersState: Triggers,
	triggersCountStore: Writable<TriggersCount | undefined>,
	target: TriggerDraftTarget,
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
		triggersState.selectedTriggerIndex = await triggersState.addDraftTrigger(
			triggersCountStore,
			triggerType,
			target
		)
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
		return { schedule: trigger.schedule, enabled: trigger.enabled, summary: trigger.summary }
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
	} else if (triggerType === 'azure') {
		return {
			azure_resource_path: trigger.azure_resource_path,
			azure_mode: trigger.azure_mode,
			scope_resource_id: trigger.scope_resource_id,
			topic_name: trigger.topic_name
		}
	} else if (triggerType === 'email') {
		return { local_part: trigger.local_part }
	} else if (triggerType === 'nextcloud') {
		return { event: trigger.service_config?.event ?? trigger.event, summary: trigger.summary }
	} else if (triggerType === 'google') {
		return {
			trigger_type: trigger.service_config?.triggerType ?? trigger.trigger_type,
			resource_id: trigger.service_config?.resourceId ?? trigger.resource_id,
			resource_name: trigger.service_config?.resourceName ?? trigger.resource_name,
			calendar_id: trigger.service_config?.calendarId ?? trigger.calendar_id,
			calendar_name: trigger.service_config?.calendarName ?? trigger.calendar_name,
			summary: trigger.summary
		}
	} else if (triggerType === 'github') {
		return {
			owner: trigger.service_config?.owner ?? trigger.owner,
			repo: trigger.service_config?.repo ?? trigger.repo,
			events: trigger.service_config?.events ?? trigger.events,
			summary: trigger.summary
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
	} else if (type === 'nextcloud' && config?.summary) {
		return `${config.summary}`
	} else if (type === 'nextcloud' && path) {
		return `${path}`
	} else if (type === 'google' && config?.summary) {
		return `${config.summary}`
	} else if (type === 'google' && path) {
		const triggerType = config?.trigger_type ?? config?.triggerType
		if (triggerType === 'calendar') {
			const name = config?.resource_name ?? config?.calendar_id ?? ''
			return `Calendar: ${name || path}`
		} else {
			const name = config?.resource_name ?? ''
			return name ? `Drive: ${name}` : config?.resource_id ? `Drive: ${path}` : `Drive: All changes`
		}
	} else if (type === 'github' && config?.summary) {
		return `${config.summary}`
	} else if (type === 'github' && config?.owner && config?.repo) {
		return `${config.owner}/${config.repo}`
	} else if (isDraft && draftConfig?.path) {
		return `${draftConfig?.path}`
	} else if (isDraft && !trigger.draftPath && !path) {
		return `New ${type.replace(/s$/, '')} trigger`
	} else {
		// A renamed draft is keyed at its original path but deploys to `draftPath`.
		return trigger.draftPath ?? path ?? ''
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
		'azure',
		'email',
		'nextcloud',
		'google',
		'github'
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

export type FlowWithDraftAndDraftTriggers = Flow
export type NewScriptWithDraftAndDraftTriggers = NewScript & { hash?: string }

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
