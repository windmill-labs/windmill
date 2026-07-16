import {
	HttpTriggerService,
	WebsocketTriggerService,
	KafkaTriggerService,
	NatsTriggerService,
	SqsTriggerService,
	MqttTriggerService,
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
 * (its list endpoint 404s on CE), and the list call itself.
 */
export const TRIGGER_KINDS: Record<
	WorkspaceTriggerKind,
	{
		badge: string
		route: string
		note?: string
		resourceField?: string
		eeOnly?: boolean
		list: (workspace: string) => Promise<Array<Record<string, any>>>
	}
> = {
	http: {
		badge: 'HTTP',
		route: 'routes',
		note: 'Webhook URL regenerates on import — re-register with the external service.',
		resourceField: 'authentication_resource_path',
		list: (workspace) => HttpTriggerService.listHttpTriggers({ workspace })
	},
	websocket: {
		badge: 'WebSocket',
		route: 'websocket_triggers',
		note: 'Reconnect WebSocket auth after import if external service requires it.',
		list: (workspace) => WebsocketTriggerService.listWebsocketTriggers({ workspace })
	},
	schedule: {
		badge: 'Schedule',
		route: 'schedules',
		list: (workspace) => ScheduleService.listSchedules({ workspace })
	},
	kafka: {
		badge: 'Kafka',
		route: 'kafka_triggers',
		note: 'Verify Kafka broker access from the importing instance.',
		resourceField: 'kafka_resource_path',
		eeOnly: true,
		list: (workspace) => KafkaTriggerService.listKafkaTriggers({ workspace })
	},
	nats: {
		badge: 'NATS',
		route: 'nats_triggers',
		note: 'Verify NATS connection from the importing instance.',
		resourceField: 'nats_resource_path',
		eeOnly: true,
		list: (workspace) => NatsTriggerService.listNatsTriggers({ workspace })
	},
	sqs: {
		badge: 'SQS',
		route: 'sqs_triggers',
		resourceField: 'aws_resource_path',
		eeOnly: true,
		list: (workspace) => SqsTriggerService.listSqsTriggers({ workspace })
	},
	mqtt: {
		badge: 'MQTT',
		route: 'mqtt_triggers',
		resourceField: 'mqtt_resource_path',
		list: (workspace) => MqttTriggerService.listMqttTriggers({ workspace })
	},
	gcp: {
		badge: 'GCP Pub/Sub',
		route: 'gcp_triggers',
		note: 'Re-link GCP Pub/Sub subscription after import.',
		resourceField: 'gcp_resource_path',
		eeOnly: true,
		list: (workspace) => GcpTriggerService.listGcpTriggers({ workspace })
	},
	azure: {
		badge: 'Azure',
		route: 'azure_triggers',
		note: 'Re-link Azure Event Grid subscription after import.',
		resourceField: 'azure_resource_path',
		eeOnly: true,
		list: (workspace) => AzureTriggerService.listAzureTriggers({ workspace })
	},
	postgres: {
		badge: 'Postgres',
		route: 'postgres_triggers',
		resourceField: 'postgres_resource_path',
		list: (workspace) => PostgresTriggerService.listPostgresTriggers({ workspace })
	},
	email: {
		badge: 'Email',
		route: 'email_triggers',
		note: 'Email address regenerates on import.',
		list: (workspace) => EmailTriggerService.listEmailTriggers({ workspace })
	}
}

export const WORKSPACE_TRIGGER_KINDS = Object.keys(TRIGGER_KINDS) as WorkspaceTriggerKind[]

/**
 * List every trigger of every kind in the workspace, normalized. EE-only kinds
 * are skipped without a license (their endpoints 404 on CE and would flood the
 * console); a kind whose call fails yields no rows rather than failing the load.
 */
export async function listAllWorkspaceTriggers(
	workspace: string,
	opts: { includeEeOnly: boolean }
): Promise<WorkspaceTrigger[]> {
	const perKind = await Promise.all(
		WORKSPACE_TRIGGER_KINDS.map(async (kind) => {
			const def = TRIGGER_KINDS[kind]
			if (def.eeOnly && !opts.includeEeOnly) return []
			let rows: Array<Record<string, any>>
			try {
				rows = await def.list(workspace)
			} catch {
				return []
			}
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
	return perKind.flat()
}

/** The resource path a trigger's config points at, if its kind has one. */
export function triggerResourcePath(t: WorkspaceTrigger): string | undefined {
	const field = TRIGGER_KINDS[t.kind]?.resourceField
	const v = field ? (t.config as any)?.[field] : undefined
	return typeof v === 'string' && v !== '' ? v : undefined
}

// Instance-side metadata that has no meaning outside the source workspace
// (ownership, error handlers, runtime state). Anything `last_*` or `captured_*`
// is also dropped.
const TRIGGER_CONFIG_BLACKLIST = new Set([
	'path',
	'script_path',
	'is_flow',
	'summary',
	'description',
	'workspace_id',
	'edited_by',
	'edited_at',
	'enabled',
	'extra_perms',
	'permissioned_as',
	'permissioned_as_email',
	'error',
	'error_handler_path',
	'error_handler_args',
	'test_runnable_args'
])
export function stripTriggerConfig(config: Record<string, unknown>): Record<string, unknown> {
	const out: Record<string, unknown> = {}
	for (const [k, v] of Object.entries(config)) {
		if (TRIGGER_CONFIG_BLACKLIST.has(k)) continue
		if (k.startsWith('last_') || k.startsWith('captured_')) continue
		out[k] = v
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
