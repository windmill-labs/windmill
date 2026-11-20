import {
	GranularAclService,
	type BrowseAccessFoldersResponse,
	type BrowseAccessItemsResponse
} from '$lib/gen'

export type GrantLevel = 'none' | 'read' | 'write'

/** Where the Grant navigator reads and changes grants. Without one it uses the API, in the
 * operating workspace. */
export type GrantSource = {
	/** The caller's own `u/` space, listed first. */
	ownSpace: string
	folders: () => Promise<BrowseAccessFoldersResponse>
	items: (query: {
		prefix?: string
		search?: string
		offset?: number
	}) => Promise<BrowseAccessItemsResponse>
	/** Sets the principal's own entry, as `setGrant` does. */
	set: (kind: string, path: string, level: GrantLevel) => Promise<void>
}

type AclKind = Parameters<typeof GranularAclService.addGranularAcls>[0]['kind']

export const KIND_LABELS: Record<string, string> = {
	script: 'Script',
	flow: 'Flow',
	app: 'App',
	raw_app: 'Full-code app',
	resource: 'Resource',
	variable: 'Variable',
	schedule: 'Schedule',
	http_trigger: 'HTTP route',
	websocket_trigger: 'WebSocket trigger',
	kafka_trigger: 'Kafka trigger',
	nats_trigger: 'NATS trigger',
	postgres_trigger: 'Postgres trigger',
	mqtt_trigger: 'MQTT trigger',
	amqp_trigger: 'AMQP trigger',
	gcp_trigger: 'GCP Pub/Sub trigger',
	azure_trigger: 'Azure trigger',
	sqs_trigger: 'SQS trigger',
	email_trigger: 'Email trigger'
}

/** Sets `principal`'s own entry on an item or folder (`kind` `folder`, `path` the folder name). */
export async function setGrant(
	workspace: string,
	kind: string,
	path: string,
	principal: string,
	level: GrantLevel
): Promise<void> {
	if (level === 'none') {
		await GranularAclService.removeGranularAcls({
			workspace,
			kind: kind as AclKind,
			path,
			requestBody: { owner: principal }
		})
	} else {
		await GranularAclService.addGranularAcls({
			workspace,
			kind: kind as AclKind,
			path,
			requestBody: { owner: principal, write: level === 'write' }
		})
	}
}
