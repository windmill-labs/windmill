import type { OffboardAffectedPaths } from '$lib/gen'

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
	nats_trigger: 'nats_triggers',
	sqs_trigger: 'sqs_triggers',
	gcp_trigger: 'gcp_triggers',
	email_trigger: 'email_triggers'
}

const TRIGGER_TABLE_TO_LABEL: Record<string, string> = {
	http_trigger: 'http trigger',
	websocket_trigger: 'websocket trigger',
	kafka_trigger: 'kafka trigger',
	postgres_trigger: 'postgres trigger',
	mqtt_trigger: 'mqtt trigger',
	nats_trigger: 'nats trigger',
	sqs_trigger: 'sqs trigger',
	gcp_trigger: 'gcp trigger',
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
		case 'scripts':
			return `/scripts/get/${path}`
		case 'flows':
			return `/flows/get/${path}`
		case 'apps':
			return `/apps/get/${path}`
		case 'resources':
			return `/resources#/resource/${path}`
		case 'variables':
			return `/variables#${path}`
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
