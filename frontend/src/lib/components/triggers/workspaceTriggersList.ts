import {
	HttpTriggerService,
	WebsocketTriggerService,
	KafkaTriggerService,
	NatsTriggerService,
	SqsTriggerService,
	MqttTriggerService,
	AmqpTriggerService,
	GcpTriggerService,
	AzureTriggerService,
	PostgresTriggerService,
	EmailTriggerService,
	ScheduleService
} from '$lib/gen'

export type WorkspaceTriggerKind =
	| 'http'
	| 'websocket'
	| 'schedule'
	| 'kafka'
	| 'nats'
	| 'sqs'
	| 'mqtt'
	| 'amqp'
	| 'gcp'
	| 'azure'
	| 'postgres'
	| 'email'

/** One trigger row, normalized across kinds; `config` keeps the raw API row. */
export interface WorkspaceTrigger {
	kind: WorkspaceTriggerKind
	path: string
	script_path: string
	is_flow: boolean
	summary?: string
	config: Record<string, unknown>
}

/**
 * Single source of truth for per-kind trigger knowledge shared by surfaces that
 * handle every trigger kind (Hub publish/install, exports): display badge, the
 * workspace list route, an optional post-import caveat, the config field
 * holding the kind's resource path, whether the kind requires an EE license
 * (its endpoints 404 on CE), and the list/create calls. Schedules have no
 * `create` entry: they disable via `enabled` (not `mode`) and use a dedicated
 * body shape, handled in `createWorkspaceTriggerDisabled`.
 */
export const TRIGGER_KINDS: Record<
	WorkspaceTriggerKind,
	{
		badge: string
		route: string
		note?: string
		resourceField?: string
		eeOnly?: boolean
		/**
		 * Creating a trigger of this kind manages external cloud state (e.g.
		 * Pub/Sub or Event Grid subscriptions) before storing it, even with
		 * `mode: 'disabled'` — so it must never be auto-created from an import.
		 */
		provisionsOnCreate?: boolean
		/**
		 * The kind-specific config fields that cross the workspace boundary
		 * (export to the Hub AND import from it) — an allowlist, so a field
		 * added upstream is dropped until someone consciously admits it here,
		 * instead of leaking to the Hub or being injectable from a crafted
		 * export. Identity/ownership/runtime fields (path, permissioned_as,
		 * email, enabled, server_id, provisioned subscription ids, …) must
		 * never be listed. `tag` (schedules) is excluded deliberately: it names
		 * a worker group of the source instance, and a foreign tag makes
		 * imported jobs queue forever on a tag no worker serves. Non-schedule
		 * kinds get the shared behavior fields (error handler, retry) appended
		 * by `portableTriggerConfig`.
		 */
		configFields: string[]
		list: (
			workspace: string,
			onError?: (message: string) => void
		) => Promise<Array<Record<string, any>>>
		create?: (workspace: string, requestBody: any) => Promise<unknown>
	}
> = {
	http: {
		configFields: [
			'route_path',
			'workspaced_route',
			'http_method',
			'authentication_resource_path',
			'is_async',
			'request_type',
			'authentication_method',
			'is_static_website',
			'static_asset_config',
			'wrap_body',
			'raw_string'
		],
		badge: 'HTTP',
		route: 'routes',
		note: 'Webhook URL regenerates on import — re-register with the external service.',
		resourceField: 'authentication_resource_path',
		list: (workspace) => HttpTriggerService.listHttpTriggers({ workspace }),
		create: (workspace, requestBody) =>
			HttpTriggerService.createHttpTrigger({ workspace, requestBody })
	},
	websocket: {
		configFields: [
			'url',
			'filters',
			'filter_logic',
			'initial_messages',
			'url_runnable_args',
			'can_return_message',
			'can_return_error_result',
			'heartbeat'
		],
		badge: 'WebSocket',
		route: 'websocket_triggers',
		note: 'Reconnect WebSocket auth after import if external service requires it.',
		list: (workspace) => WebsocketTriggerService.listWebsocketTriggers({ workspace }),
		create: (workspace, requestBody) =>
			WebsocketTriggerService.createWebsocketTrigger({ workspace, requestBody })
	},
	schedule: {
		configFields: [
			'schedule',
			'timezone',
			'args',
			'on_failure',
			'on_failure_times',
			'on_failure_exact',
			'on_failure_extra_args',
			'on_recovery',
			'on_recovery_times',
			'on_recovery_extra_args',
			'on_success',
			'on_success_extra_args',
			'ws_error_handler_muted',
			'retry',
			'no_flow_overlap',
			'cron_version',
			'dynamic_skip'
		],
		badge: 'Schedule',
		route: 'schedules',
		// listSchedules returns slim rows (no args, handlers, cron_version, retry,
		// no_flow_overlap) — resolve each to the full schedule so exported configs
		// are complete. A schedule whose detail fetch fails is excluded and
		// reported: exporting the slim row would silently publish a schedule with
		// default behavior instead of its real settings.
		list: async (workspace, onError) => {
			const rows = await ScheduleService.listSchedules({ workspace })
			const full = await Promise.all(
				rows.map((r) =>
					ScheduleService.getSchedule({ workspace, path: r.path }).catch((e: any) => {
						onError?.(
							`Failed to load schedule ${r.path} (${e?.message ?? e}) — excluded from the project`
						)
						return undefined
					})
				)
			)
			return full.filter((r): r is NonNullable<typeof r> => r !== undefined)
		}
	},
	kafka: {
		configFields: [
			'kafka_resource_path',
			'group_id',
			'topics',
			'filters',
			'filter_logic',
			'auto_offset_reset',
			'auto_commit'
		],
		badge: 'Kafka',
		route: 'kafka_triggers',
		note: 'Verify Kafka broker access from the importing instance.',
		resourceField: 'kafka_resource_path',
		eeOnly: true,
		list: (workspace) => KafkaTriggerService.listKafkaTriggers({ workspace }),
		create: (workspace, requestBody) =>
			KafkaTriggerService.createKafkaTrigger({ workspace, requestBody })
	},
	nats: {
		configFields: [
			'nats_resource_path',
			'use_jetstream',
			'stream_name',
			'consumer_name',
			'subjects'
		],
		badge: 'NATS',
		route: 'nats_triggers',
		note: 'Verify NATS connection from the importing instance.',
		resourceField: 'nats_resource_path',
		eeOnly: true,
		list: (workspace) => NatsTriggerService.listNatsTriggers({ workspace }),
		create: (workspace, requestBody) =>
			NatsTriggerService.createNatsTrigger({ workspace, requestBody })
	},
	sqs: {
		configFields: [
			'queue_url',
			'aws_auth_resource_type',
			'aws_resource_path',
			'message_attributes'
		],
		badge: 'SQS',
		route: 'sqs_triggers',
		resourceField: 'aws_resource_path',
		eeOnly: true,
		list: (workspace) => SqsTriggerService.listSqsTriggers({ workspace }),
		create: (workspace, requestBody) =>
			SqsTriggerService.createSqsTrigger({ workspace, requestBody })
	},
	mqtt: {
		configFields: [
			'mqtt_resource_path',
			'subscribe_topics',
			'client_id',
			'v3_config',
			'v5_config',
			'client_version'
		],
		badge: 'MQTT',
		route: 'mqtt_triggers',
		resourceField: 'mqtt_resource_path',
		list: (workspace) => MqttTriggerService.listMqttTriggers({ workspace }),
		create: (workspace, requestBody) =>
			MqttTriggerService.createMqttTrigger({ workspace, requestBody })
	},
	amqp: {
		configFields: ['amqp_resource_path', 'queue_name', 'exchange', 'options'],
		badge: 'AMQP',
		route: 'amqp_triggers',
		note: 'Verify AMQP broker access from the importing instance.',
		resourceField: 'amqp_resource_path',
		list: (workspace) => AmqpTriggerService.listAmqpTriggers({ workspace }),
		create: (workspace, requestBody) =>
			AmqpTriggerService.createAmqpTrigger({ workspace, requestBody })
	},
	gcp: {
		configFields: ['gcp_resource_path', 'topic_id', 'delivery_type', 'subscription_mode'],
		provisionsOnCreate: true,
		badge: 'GCP Pub/Sub',
		route: 'gcp_triggers',
		note: 'Re-link GCP Pub/Sub subscription after import.',
		resourceField: 'gcp_resource_path',
		eeOnly: true,
		list: (workspace) => GcpTriggerService.listGcpTriggers({ workspace }),
		create: (workspace, requestBody) =>
			GcpTriggerService.createGcpTrigger({ workspace, requestBody })
	},
	azure: {
		configFields: [
			'azure_resource_path',
			'azure_mode',
			'scope_resource_id',
			'topic_name',
			'event_type_filters'
		],
		provisionsOnCreate: true,
		badge: 'Azure',
		route: 'azure_triggers',
		note: 'Re-link Azure Event Grid subscription after import.',
		resourceField: 'azure_resource_path',
		eeOnly: true,
		list: (workspace) => AzureTriggerService.listAzureTriggers({ workspace }),
		create: (workspace, requestBody) =>
			AzureTriggerService.createAzureTrigger({ workspace, requestBody })
	},
	postgres: {
		configFields: [
			'postgres_resource_path',
			'replication_slot_name',
			'publication_name',
			'publication'
		],
		badge: 'Postgres',
		route: 'postgres_triggers',
		resourceField: 'postgres_resource_path',
		list: (workspace) => PostgresTriggerService.listPostgresTriggers({ workspace }),
		create: (workspace, requestBody) =>
			PostgresTriggerService.createPostgresTrigger({ workspace, requestBody })
	},
	email: {
		configFields: ['local_part', 'workspaced_local_part'],
		badge: 'Email',
		route: 'email_triggers',
		note: 'Email address regenerates on import.',
		list: (workspace) => EmailTriggerService.listEmailTriggers({ workspace }),
		create: (workspace, requestBody) =>
			EmailTriggerService.createEmailTrigger({ workspace, requestBody })
	}
}

export const WORKSPACE_TRIGGER_KINDS = Object.keys(TRIGGER_KINDS) as WorkspaceTriggerKind[]

export interface WorkspaceTriggersListing {
	triggers: WorkspaceTrigger[]
	/**
	 * Kinds whose discovery failed or was incomplete. A 404 on a kind's list
	 * endpoint is NOT a failure (the instance doesn't have that trigger feature
	 * compiled in); anything else means triggers of that kind may exist but
	 * couldn't be enumerated, so consumers must not treat the listing as a
	 * complete snapshot (e.g. publishing should be blocked until a retry).
	 */
	failedKinds: WorkspaceTriggerKind[]
}

/**
 * List every trigger of every kind in the workspace, normalized. EE-only kinds
 * are skipped without a license (their endpoints 404 on CE and would flood the
 * console). Failures are reported through `opts.onError` and recorded in
 * `failedKinds` rather than silently yielding an empty kind.
 */
export async function listAllWorkspaceTriggers(
	workspace: string,
	opts: { includeEeOnly: boolean; onError?: (message: string) => void }
): Promise<WorkspaceTriggersListing> {
	const failedKinds: WorkspaceTriggerKind[] = []
	const perKind = await Promise.all(
		WORKSPACE_TRIGGER_KINDS.map(async (kind) => {
			const def = TRIGGER_KINDS[kind]
			if (def.eeOnly && !opts.includeEeOnly) return []
			let kindFailed = false
			const reportError = (message: string) => {
				kindFailed = true
				opts.onError?.(message)
			}
			let rows: Array<Record<string, any>>
			try {
				rows = await def.list(workspace, reportError)
			} catch (e: any) {
				if (e?.status !== 404) {
					reportError(`Failed to list ${kind} triggers: ${e?.message ?? e}`)
				}
				rows = []
			}
			if (kindFailed) failedKinds.push(kind)
			return rows.map(
				(r): WorkspaceTrigger => ({
					kind,
					path: r.path,
					script_path: r.script_path,
					is_flow: r.is_flow ?? false,
					summary: r.summary,
					config: r
				})
			)
		})
	)
	return { triggers: perKind.flat(), failedKinds }
}

/**
 * Create a trigger in its disabled state — the semantics differ per kind
 * (schedules use `enabled: false`, every other kind uses `mode: 'disabled'`)
 * and are encoded here once so importers can't get them wrong.
 */
export async function createWorkspaceTriggerDisabled(
	workspace: string,
	trigger: {
		kind: string
		path: string
		script_path: string
		is_flow: boolean
		summary?: string | null
		config?: Record<string, any> | null
	},
	opts: { hasEeLicense: boolean }
): Promise<unknown> {
	const def = TRIGGER_KINDS[trigger.kind as WorkspaceTriggerKind]
	if (!def) throw new Error(`trigger kind '${trigger.kind}' not supported yet`)
	if (def.eeOnly && !opts.hasEeLicense) {
		throw new Error(`trigger kind '${trigger.kind}' requires Enterprise`)
	}
	if (def.provisionsOnCreate) {
		throw new Error(
			`${def.badge} triggers manage cloud subscriptions at creation — fill in the imported resource, then re-create this trigger manually`
		)
	}
	// Remote input: only the allowlisted portable slice may reach the create call.
	const config = portableTriggerConfig(trigger.kind, trigger.config)
	if (trigger.kind === 'schedule') {
		// Spread the portable config first so behavioral settings survive the
		// import (cron_version, retry, failure/recovery/success handlers,
		// no_flow_overlap, …) — restoring only cron+timezone would silently
		// change the schedule's semantics once re-enabled.
		return ScheduleService.createSchedule({
			workspace,
			requestBody: {
				...config,
				path: trigger.path,
				schedule: (config.schedule as string) ?? '0 0 * * * *',
				timezone: (config.timezone as string) ?? 'UTC',
				script_path: trigger.script_path,
				is_flow: trigger.is_flow,
				enabled: false,
				args: (config.args as any) ?? {},
				summary: trigger.summary ?? null
			}
		})
	}
	// `config` holds only allowlisted kind-specific fields; explicit fields win.
	return def.create!(workspace, {
		...config,
		path: trigger.path,
		script_path: trigger.script_path,
		is_flow: trigger.is_flow,
		summary: trigger.summary ?? null,
		mode: 'disabled'
	})
}

/** The resource path a trigger's config points at, if its kind has one. */
export function triggerResourcePath(t: WorkspaceTrigger): string | undefined {
	const field = TRIGGER_KINDS[t.kind]?.resourceField
	const v = field ? (t.config as any)?.[field] : undefined
	return typeof v === 'string' && v !== '' ? v : undefined
}

/**
 * Runnables a trigger's config references beyond its primary target, so they
 * can be bundled alongside it: `error_handler_path` (bare script path) for
 * non-schedule kinds, and schedules' `on_failure`/`on_recovery`/`on_success`
 * (`script/<path>` or `flow/<path>`) plus `dynamic_skip` (bare script path —
 * schedule creation refuses a dynamic_skip whose script doesn't exist, so an
 * unbundled one would make the import fail).
 */
export function triggerHandlerRefs(
	t: WorkspaceTrigger
): Array<{ kind: 'script' | 'flow'; path: string }> {
	const c = t.config as any
	const out: Array<{ kind: 'script' | 'flow'; path: string }> = []
	if (t.kind === 'schedule') {
		for (const field of ['on_failure', 'on_recovery', 'on_success']) {
			const v = c?.[field]
			const m = typeof v === 'string' ? /^(script|flow)\/(.+)$/.exec(v) : null
			if (m) out.push({ kind: m[1] as 'script' | 'flow', path: m[2] })
		}
		if (typeof c?.dynamic_skip === 'string' && c.dynamic_skip !== '') {
			out.push({ kind: 'script', path: c.dynamic_skip })
		}
	} else {
		if (typeof c?.error_handler_path === 'string' && c.error_handler_path !== '') {
			out.push({ kind: 'script', path: c.error_handler_path })
		}
		if (t.kind === 'websocket') {
			// The URL itself can be a runnable ($script:<path> / $flow:<path>), and
			// initial messages can be runnable results.
			const um = typeof c?.url === 'string' ? /^\$(script|flow):(.+)$/.exec(c.url) : null
			if (um) out.push({ kind: um[1] as 'script' | 'flow', path: um[2] })
			for (const msg of Array.isArray(c?.initial_messages) ? c.initial_messages : []) {
				const rr = msg?.runnable_result
				if (rr && typeof rr.path === 'string' && rr.path !== '') {
					out.push({ kind: rr.is_flow ? 'flow' : 'script', path: rr.path })
				}
			}
		}
	}
	return out
}

// Instance-side metadata that has no meaning outside the source workspace
// (ownership, runtime state). Anything `last_*` or `captured_*` is also
// dropped. Error handlers (error_handler_path/args, schedules' on_* fields)
// are functional config and stay: their runnables are bundled with the
// project and the paths relocated.
// Portable behavior fields shared by every non-schedule kind (schedules carry
// their own handler fields in configFields).
const COMMON_CONFIG_FIELDS = ['error_handler_path', 'error_handler_args', 'retry']

/**
 * The portable slice of a trigger config — the ONLY fields that cross the
 * workspace boundary, in either direction. Applied on export (what reaches
 * the Hub) and on import (what a Hub export may feed into a create call), so
 * an upstream field addition can't leak out, and a crafted export can't
 * inject non-allowlisted fields like `permissioned_as`.
 */
export function portableTriggerConfig(
	kind: string,
	config: Record<string, unknown> | null | undefined
): Record<string, unknown> {
	const def = TRIGGER_KINDS[kind as WorkspaceTriggerKind]
	if (!def || !config) return {}
	const fields =
		kind === 'schedule' ? def.configFields : [...def.configFields, ...COMMON_CONFIG_FIELDS]
	const out: Record<string, unknown> = {}
	for (const f of fields) {
		if (config[f] !== undefined) out[f] = config[f]
	}
	return out
}

/** Human-readable config facts per kind, for detail panes. */
export function triggerDetails(t: WorkspaceTrigger): Array<{ label: string; value: string }> {
	const c = t.config as any
	const out: Array<{ label: string; value: string }> = []
	const push = (label: string, v: any) => {
		if (v != null && v !== '') out.push({ label, value: String(v) })
	}
	switch (t.kind) {
		case 'http':
			push('Route', `${(c.http_method ?? '').toUpperCase()} /${c.route_path ?? ''}`)
			push('Auth', c.authentication_method)
			break
		case 'schedule':
			push('Cron', c.schedule)
			push('Timezone', c.timezone)
			break
		case 'websocket':
			push('URL', c.url)
			break
		case 'kafka':
			push('Resource', c.kafka_resource_path)
			push('Group', c.group_id)
			push('Topics', (Array.isArray(c.topics) ? c.topics : []).join(', '))
			break
		case 'nats':
			push('Resource', c.nats_resource_path)
			push('Subjects', (Array.isArray(c.subjects) ? c.subjects : []).join(', '))
			push('Jetstream', c.use_jetstream)
			break
		case 'sqs':
			push('Queue', c.queue_url)
			push('Resource', c.aws_resource_path)
			break
		case 'mqtt':
			push('Resource', c.mqtt_resource_path)
			push(
				'Topics',
				(Array.isArray(c.subscribe_topics) ? c.subscribe_topics : [])
					.map((x: any) => x?.topic ?? x)
					.join(', ')
			)
			break
		case 'amqp':
			push('Resource', c.amqp_resource_path)
			push('Queue', c.queue_name)
			break
		case 'gcp':
			push('Resource', c.gcp_resource_path)
			push('Topic', c.topic_id)
			push('Subscription', c.subscription_id)
			break
		case 'azure':
			push('Resource', c.azure_resource_path)
			push('Scope', c.scope_resource_id)
			push('Subscription', c.subscription_name)
			break
		case 'postgres':
			push('Resource', c.postgres_resource_path)
			push('Publication', c.publication_name)
			break
		case 'email':
			push('Email prefix', c.local_part ? `${c.local_part}@…` : undefined)
			break
	}
	return out
}
