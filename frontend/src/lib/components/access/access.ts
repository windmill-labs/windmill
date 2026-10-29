import { GranularAclService } from '$lib/gen'

export type GrantLevel = 'none' | 'read' | 'write'

type AclKind = Parameters<typeof GranularAclService.addGranularAcls>[0]['kind']

export const KIND_LABELS: Record<string, string> = {
	script: 'Script',
	flow: 'Flow',
	app: 'App',
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
