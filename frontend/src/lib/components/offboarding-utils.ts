import {
	AiEvalsService,
	AmqpTriggerService,
	AppService,
	AzureTriggerService,
	EmailTriggerService,
	FlowService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService,
	type OffboardAffectedPaths,
	type OffboardPathConflict
} from '$lib/gen'

export function pl(n: number, singular: string): string {
	return `${n} ${singular}${n === 1 ? '' : 's'}`
}

function triggerCount(triggers: OffboardAffectedPaths['triggers']): number {
	if (!triggers) return 0
	return Object.values(triggers).reduce((s, arr) => s + arr.length, 0)
}

export function countPaths(p: OffboardAffectedPaths | undefined | null): number {
	if (!p) return 0
	return (
		(p.scripts?.length ?? 0) +
		(p.flows?.length ?? 0) +
		(p.apps?.length ?? 0) +
		(p.resources?.length ?? 0) +
		(p.variables?.length ?? 0) +
		(p.schedules?.length ?? 0) +
		triggerCount(p.triggers)
	)
}

const TRIGGER_TABLE_TO_ROUTE: Record<string, string> = {
	http_trigger: 'routes',
	websocket_trigger: 'websocket_triggers',
	kafka_trigger: 'kafka_triggers',
	postgres_trigger: 'postgres_triggers',
	mqtt_trigger: 'mqtt_triggers',
	amqp_trigger: 'amqp_triggers',
	nats_trigger: 'nats_triggers',
	sqs_trigger: 'sqs_triggers',
	gcp_trigger: 'gcp_triggers',
	azure_trigger: 'azure_triggers',
	email_trigger: 'email_triggers'
}

const TRIGGER_TABLE_TO_LABEL: Record<string, string> = {
	http_trigger: 'http trigger',
	websocket_trigger: 'websocket trigger',
	kafka_trigger: 'kafka trigger',
	postgres_trigger: 'postgres trigger',
	mqtt_trigger: 'mqtt trigger',
	amqp_trigger: 'amqp trigger',
	nats_trigger: 'nats trigger',
	sqs_trigger: 'sqs trigger',
	gcp_trigger: 'gcp trigger',
	azure_trigger: 'azure trigger',
	email_trigger: 'email trigger'
}

export function flattenPaths(
	p: OffboardAffectedPaths | undefined | null
): Array<{ kind: string; path: string }> {
	if (!p) return []
	const result: Array<{ kind: string; path: string }> = []
	for (const [kind, list] of Object.entries(p)) {
		if (kind === 'triggers' && list && typeof list === 'object' && !Array.isArray(list)) {
			for (const [triggerType, paths] of Object.entries(list as Record<string, string[]>)) {
				for (const path of paths) result.push({ kind: triggerType, path })
			}
		} else if (Array.isArray(list)) {
			for (const path of list) result.push({ kind, path })
		}
	}
	return result
}

export function triggerLabel(triggerType: string): string {
	return TRIGGER_TABLE_TO_LABEL[triggerType] ?? triggerType
}

export function kindLabel(kind: string): string {
	if (TRIGGER_TABLE_TO_LABEL[kind]) return TRIGGER_TABLE_TO_LABEL[kind]
	// "scripts" -> "script", "flows" -> "flow", etc.
	return kind.replace(/s$/, '')
}

export function itemHref(kind: string, path: string): string | undefined {
	switch (kind) {
		case 'script':
		case 'scripts':
			return `/scripts/get/${path}`
		case 'flow':
		case 'flows':
			return `/flows/get/${path}`
		case 'app':
		case 'apps':
			return `/apps/get/${path}`
		case 'resource':
		case 'resources':
			return `/resources#/resource/${path}`
		case 'variable':
		case 'variables':
			return `/variables#${path}`
		case 'schedule':
		case 'schedules':
			return `/schedules#${path}`
		default: {
			const route = TRIGGER_TABLE_TO_ROUTE[kind]
			if (route) return `/${route}#${path}`
			return undefined
		}
	}
}

export function downloadCsv(rows: string[][], filename: string) {
	const csv = rows.map((r) => r.map((c) => `"${c.replace(/"/g, '""')}"`).join(',')).join('\n')
	const blob = new Blob([csv], { type: 'text/csv' })
	const url = URL.createObjectURL(blob)
	const a = document.createElement('a')
	a.href = url
	a.download = filename
	a.click()
	URL.revokeObjectURL(url)
}

type DeleteFn = (p: { workspace: string; path: string }) => Promise<unknown>

// Keyed by the table names the offboarding conflict check reports. Scripts and flows are
// deleted rather than archived: an archived item still holds its path and keeps the conflict.
const DELETE_BY_KIND: Record<string, DeleteFn> = {
	script: (p) => ScriptService.deleteScriptByPath(p),
	flow: (p) => FlowService.deleteFlowByPath(p),
	app: (p) => AppService.deleteApp(p),
	resource: (p) => ResourceService.deleteResource(p),
	variable: (p) => VariableService.deleteVariable(p),
	schedule: (p) => ScheduleService.deleteSchedule(p),
	eval_dataset: (p) => AiEvalsService.deleteEvalDataset(p),
	http_trigger: (p) => HttpTriggerService.deleteHttpTrigger(p),
	websocket_trigger: (p) => WebsocketTriggerService.deleteWebsocketTrigger(p),
	kafka_trigger: (p) => KafkaTriggerService.deleteKafkaTrigger(p),
	postgres_trigger: (p) => PostgresTriggerService.deletePostgresTrigger(p),
	mqtt_trigger: (p) => MqttTriggerService.deleteMqttTrigger(p),
	amqp_trigger: (p) => AmqpTriggerService.deleteAmqpTrigger(p),
	nats_trigger: (p) => NatsTriggerService.deleteNatsTrigger(p),
	sqs_trigger: (p) => SqsTriggerService.deleteSqsTrigger(p),
	gcp_trigger: (p) => GcpTriggerService.deleteGcpTrigger(p),
	azure_trigger: (p) => AzureTriggerService.deleteAzureTrigger(p),
	email_trigger: (p) => EmailTriggerService.deleteEmailTrigger(p)
}

const TRASHED_KINDS = new Set(['script', 'flow', 'app', 'resource', 'variable', 'schedule'])

export function canDeleteKind(kind: string): boolean {
	return kind in DELETE_BY_KIND
}

export function isTrashedKind(kind: string): boolean {
	return TRASHED_KINDS.has(kind)
}

export async function deleteItemOfKind(
	kind: string,
	workspace: string,
	path: string
): Promise<void> {
	const del = DELETE_BY_KIND[kind]
	if (!del) throw new Error(`Cannot delete items of kind ${kind}`)
	await del({ workspace, path })
}

export function withoutConflict(
	conflicts: OffboardPathConflict[],
	resolved: OffboardPathConflict
): OffboardPathConflict[] {
	return conflicts.filter(
		(c) =>
			!(
				c.workspace_id === resolved.workspace_id &&
				c.kind === resolved.kind &&
				c.source_path === resolved.source_path
			)
	)
}

/**
 * Deleting a resource also deletes the variables linked to it, which can remove one side of a
 * variable conflict. Returns the remaining conflicts that no longer have both sides, so the list
 * does not offer to "keep" an item that is already gone.
 */
export async function conflictsGoneAfterDelete(
	deletedKind: string,
	remaining: OffboardPathConflict[]
): Promise<OffboardPathConflict[]> {
	if (deletedKind !== 'resource') return []
	const variables = remaining.filter((c) => c.kind === 'variable')
	const gone = await Promise.all(
		variables.map(async (c) => {
			const [source, target] = await Promise.all([
				VariableService.existsVariable({ workspace: c.workspace_id, path: c.source_path }),
				VariableService.existsVariable({ workspace: c.workspace_id, path: c.target_path })
			])
			return !source || !target
		})
	)
	return variables.filter((_, i) => gone[i])
}
