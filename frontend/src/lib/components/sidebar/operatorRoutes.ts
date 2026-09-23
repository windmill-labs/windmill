import { base } from '$lib/base'
import type { UserWorkspace } from '$lib/stores'

/**
 * The pages an operator can reach, each paired with the `operator_settings` key
 * that admits them — the operator menu lists only the entries the active
 * workspace permits.
 */
export type OperatorMenuLink = {
	label: string
	/** `operator_settings` key gating the page. */
	id: string
	href: string
}

export type OperatorTriggerLink = OperatorMenuLink & { kind: string }

/** Home needs no setting — it is the one page every operator always has. */
export const OPERATOR_MAIN_LINKS: OperatorMenuLink[] = [
	{ label: 'Home', id: 'home', href: `${base}/` },
	{ label: 'Runs', id: 'runs', href: `${base}/runs` },
	{ label: 'Schedules', id: 'schedules', href: `${base}/schedules` }
]

export const OPERATOR_SECONDARY_LINKS: OperatorMenuLink[] = [
	{ label: 'Resources', id: 'resources', href: `${base}/resources` },
	{ label: 'Variables', id: 'variables', href: `${base}/variables` },
	{ label: 'Assets', id: 'assets', href: `${base}/assets` },
	{ label: 'Groups', id: 'groups', href: `${base}/groups` },
	{ label: 'Folders', id: 'folders', href: `${base}/folders` },
	{ label: 'Workers', id: 'workers', href: `${base}/workers` },
	{ label: 'Audit logs', id: 'audit_logs', href: `${base}/audit_logs` }
]

export const OPERATOR_TRIGGER_LINKS: OperatorTriggerLink[] = [
	{ label: 'Custom HTTP routes', id: 'triggers', href: `${base}/routes`, kind: 'http' },
	{ label: 'Websocket triggers', id: 'triggers', href: `${base}/websocket_triggers`, kind: 'ws' },
	{
		label: 'Postgres triggers',
		id: 'triggers',
		href: `${base}/postgres_triggers`,
		kind: 'postgres'
	},
	{ label: 'Kafka triggers', id: 'triggers', href: `${base}/kafka_triggers`, kind: 'kafka' },
	{ label: 'NATS triggers', id: 'triggers', href: `${base}/nats_triggers`, kind: 'nats' },
	{ label: 'SQS triggers', id: 'triggers', href: `${base}/sqs_triggers`, kind: 'sqs' },
	{ label: 'GCP Pub/Sub triggers', id: 'triggers', href: `${base}/gcp_triggers`, kind: 'gcp' },
	{
		label: 'Azure Event Grid triggers',
		id: 'triggers',
		href: `${base}/azure_triggers`,
		kind: 'azure'
	},
	{ label: 'MQTT triggers', id: 'triggers', href: `${base}/mqtt_triggers`, kind: 'mqtt' },
	{ label: 'AMQP triggers', id: 'triggers', href: `${base}/amqp_triggers`, kind: 'amqp' },
	{ label: 'Email triggers', id: 'triggers', href: `${base}/email_triggers`, kind: 'email' }
]

/**
 * Whether the user is an operator in `workspace`. The server nulls
 * `operator_settings` for a non-operator, so a non-null value narrows an operator
 * correctly; the converse does not hold — the column is itself nullable, so a
 * genuine operator whose workspace never had settings written also reads as NULL.
 * Only rely on `true`, never on `false` meaning "not an operator".
 */
export function isOperatorInWorkspace(workspace: UserWorkspace | undefined): boolean {
	return workspace?.operator_settings != null
}
