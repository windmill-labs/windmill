import {
	AmqpTriggerService,
	AzureTriggerService,
	EmailTriggerService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	SqsTriggerService,
	WebsocketTriggerService,
	type AmqpTrigger,
	type AzureTrigger,
	type EmailTrigger,
	type GcpTrigger,
	type HttpTrigger,
	type KafkaTrigger,
	type MqttTrigger,
	type NatsTrigger,
	type PostgresTrigger,
	type SqsTrigger,
	type TriggerMode,
	type UserDraftItemKind,
	type WebsocketTrigger
} from '$lib/gen'
import type { TriggerKind as UsedTriggerKind } from '$lib/components/triggers'
import type { TriggerKind } from '$lib/components/sessions/previewPaths'

type AnyTrigger =
	| HttpTrigger
	| WebsocketTrigger
	| PostgresTrigger
	| KafkaTrigger
	| NatsTrigger
	| MqttTrigger
	| AmqpTrigger
	| SqsTrigger
	| GcpTrigger
	| AzureTrigger
	| EmailTrigger
type KeysOfUnion<T> = T extends unknown ? keyof T : never
type FieldOf<T, K extends PropertyKey> = T extends unknown
	? K extends keyof T
		? T[K]
		: never
	: never

/** A row of any trigger list: the fields every kind shares, plus every kind's own as optional,
 * so a field read by one kind's branch is still checked against the generated types. */
export type TriggerRow = Pick<
	HttpTrigger,
	'path' | 'script_path' | 'is_flow' | 'mode' | 'edited_by' | 'edited_at'
> & {
	[K in Exclude<
		KeysOfUnion<AnyTrigger>,
		'path' | 'script_path' | 'is_flow' | 'mode' | 'edited_by' | 'edited_at'
	>]?: FieldOf<AnyTrigger, K>
} & { draft_only?: boolean; is_draft?: boolean }

type Call<A> = (a: A) => Promise<unknown>

export type TriggerListConfig = {
	/** Page title, and the kind's plural name in the cloud alert. */
	title: string
	tooltip: string
	documentationLink?: string
	/** Follows "New" on the create button. */
	newLabel: string
	searchPlaceholder: string
	/** The "Filter by path of" choice naming the trigger itself. */
	filterLabel: string
	empty: {
		title: string
		description: string
		actionLabel: string
		aiId: string
		aiDescription: string
	}
	/** Disabled in the multi-tenant cloud. */
	cloudDisabled: boolean
	/** Hidden from operators unless the workspace grants them triggers. */
	operatorGate: boolean
	/** Creating and deleting are for admins only. */
	adminOnly: boolean
	usedKind: UsedTriggerKind
	deployKind: UsedTriggerKind
	shareKind: string
	draftKind: UserDraftItemKind
	/** Kinds holding a live connection: polled, with a status dot. Each message is followed by
	 * the error or state it reports. */
	status?: {
		starting: string
		notConnected: string
		disabled: string
		connected: string
		/** Reports a connection whose server is going away. */
		shuttingDown: boolean
		/** The server stamps a ping; without one, a connected server counts as alive. */
		pings: boolean
	}
	/** Mode changes run through the fork-conflict check, under this name. Kinds whose runtime
	 * address is workspace-prefixed skip it: a fork and its parent never collide. */
	forkConflictLabel?: string
	/** Toast a mode change that went through, as "<Mode> <label> <path>". */
	modeToastLabel?: string
	/** A failed mode change, followed by ": <error>". */
	modeError: (verb: 'enable' | 'disable' | 'suspend') => string
	/** Report a delete through a toast, as "Successfully deleted <label>: <path>". */
	deleteToastLabel?: string
	/** Report a failed delete through a toast. */
	catchDelete: boolean
	list: Call<{ workspace: string; includeDraftOnly?: boolean }>
	setMode: Call<{
		workspace: string
		path: string
		requestBody: { mode: TriggerMode; force?: boolean }
	}>
	remove: Call<{ workspace: string; path: string }>
}

const brokerTooltip = (name: string) =>
	`Windmill can connect to an ${name} broker, subscribe to specific topics, and trigger scripts or flows based on those topics.`

const consumerStatus = {
	starting: 'Consumer is starting...',
	notConnected: 'Consumer is not connected',
	disabled: 'Consumer was disabled because of an error',
	connected: 'Consumer is connected',
	shuttingDown: false,
	pings: true
}

export const TRIGGER_LIST_CONFIG: Record<TriggerKind, TriggerListConfig> = {
	http: {
		title: 'Custom HTTP routes',
		tooltip:
			'Every script and flow already has a canonical HTTP API endpoint/webhook attached to it, this is to create additional parametrizable ones.',
		documentationLink: 'https://www.windmill.dev/docs/core_concepts/http_routing',
		newLabel: 'route',
		searchPlaceholder: 'Search routes',
		filterLabel: 'Route',
		empty: {
			title: 'No custom HTTP routes yet',
			description:
				'Every script and flow already has a canonical HTTP endpoint attached to it. These are additional parametrizable ones.',
			actionLabel: 'Add a route',
			aiId: 'routes-empty-add',
			aiDescription: 'Add route'
		},
		cloudDisabled: false,
		operatorGate: true,
		adminOnly: false,
		usedKind: 'routes',
		deployKind: 'routes',
		shareKind: 'http_trigger',
		draftKind: 'trigger_http',
		modeError: (verb) => `Cannot ${verb} http trigger`,
		deleteToastLabel: 'HTTP route',
		catchDelete: true,
		list: (a) => HttpTriggerService.listHttpTriggers(a),
		setMode: (a) => HttpTriggerService.setHttpTriggerMode(a),
		remove: (a) => HttpTriggerService.deleteHttpTrigger(a)
	},
	websocket: {
		title: 'WebSocket triggers',
		tooltip: 'Windmill can listen to WebSocket events and trigger scripts or flows based on them.',
		newLabel: 'WebSocket trigger',
		searchPlaceholder: 'Search WS triggers',
		filterLabel: 'WS trigger',
		empty: {
			title: 'No WebSocket triggers yet',
			description:
				'Windmill can listen to WebSocket events and trigger scripts or flows based on them.',
			actionLabel: 'Add a WebSocket trigger',
			aiId: 'websocket-triggers-empty-add',
			aiDescription: 'Add WebSocket trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'websockets',
		deployKind: 'websockets',
		shareKind: 'websocket_trigger',
		draftKind: 'trigger_websocket',
		status: {
			starting: 'WebSocket is starting...',
			notConnected: 'WebSocket is not connected',
			disabled: 'WebSocket was disabled because of an error',
			connected: 'WebSocket is connected',
			shuttingDown: true,
			pings: true
		},
		forkConflictLabel: 'websocket trigger',
		modeToastLabel: 'websocket trigger',
		modeError: (verb) => `Cannot ${verb} websocket trigger`,
		catchDelete: false,
		list: (a) => WebsocketTriggerService.listWebsocketTriggers(a),
		setMode: (a) => WebsocketTriggerService.setWebsocketTriggerMode(a),
		remove: (a) => WebsocketTriggerService.deleteWebsocketTrigger(a)
	},
	postgres: {
		title: 'Postgres triggers',
		tooltip:
			'Windmill enables real-time responsiveness by listening to specific database transactions—such as inserts, updates, and deletes—and automatically triggering scripts or workflows in response.',
		newLabel: 'Postgres trigger',
		searchPlaceholder: 'Search Postgres triggers',
		filterLabel: 'Postgres trigger',
		empty: {
			title: 'No Postgres triggers yet',
			description:
				'Windmill can listen to database transactions — inserts, updates and deletes — and trigger scripts or flows in response.',
			actionLabel: 'Add a Postgres trigger',
			aiId: 'postgres-triggers-empty-add',
			aiDescription: 'Add Postgres trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'postgres',
		deployKind: 'postgres',
		shareKind: 'postgres_trigger',
		draftKind: 'trigger_postgres',
		status: {
			starting: 'Postgres trigger is starting...',
			notConnected: 'Could not connect to database',
			disabled: 'Disabled because of an error',
			connected: 'Connected to database',
			shuttingDown: true,
			pings: false
		},
		forkConflictLabel: 'postgres trigger',
		modeToastLabel: 'postgres trigger',
		modeError: () => 'Cannot change postgres trigger mode',
		catchDelete: false,
		list: (a) => PostgresTriggerService.listPostgresTriggers(a),
		setMode: (a) => PostgresTriggerService.setPostgresTriggerMode(a),
		remove: (a) => PostgresTriggerService.deletePostgresTrigger(a)
	},
	kafka: {
		title: 'Kafka triggers',
		tooltip: 'Windmill can consume kafka events and trigger scripts or flows based on them.',
		newLabel: 'Kafka trigger',
		searchPlaceholder: 'Search Kafka triggers',
		filterLabel: 'Kafka trigger',
		empty: {
			title: 'No Kafka triggers yet',
			description: 'Windmill can consume kafka events and trigger scripts or flows based on them.',
			actionLabel: 'Add a Kafka trigger',
			aiId: 'kafka-triggers-empty-add',
			aiDescription: 'Add Kafka trigger'
		},
		cloudDisabled: true,
		operatorGate: true,
		adminOnly: false,
		usedKind: 'kafka',
		deployKind: 'kafka',
		shareKind: 'kafka_trigger',
		draftKind: 'trigger_kafka',
		status: consumerStatus,
		forkConflictLabel: 'Kafka trigger',
		modeError: (verb) => `Cannot ${verb} Kafka trigger`,
		catchDelete: false,
		list: (a) => KafkaTriggerService.listKafkaTriggers(a),
		setMode: (a) => KafkaTriggerService.setKafkaTriggerMode(a),
		remove: (a) => KafkaTriggerService.deleteKafkaTrigger(a)
	},
	nats: {
		title: 'NATS triggers',
		tooltip: 'Windmill can consume NATS events and trigger scripts or flows based on them.',
		newLabel: 'NATS trigger',
		searchPlaceholder: 'Search NATS triggers',
		filterLabel: 'NATS trigger',
		empty: {
			title: 'No NATS triggers yet',
			description: 'Windmill can consume NATS events and trigger scripts or flows based on them.',
			actionLabel: 'Add a NATS trigger',
			aiId: 'nats-triggers-empty-add',
			aiDescription: 'Add NATS trigger'
		},
		cloudDisabled: true,
		operatorGate: true,
		adminOnly: false,
		usedKind: 'nats',
		deployKind: 'nats',
		shareKind: 'nats_trigger',
		draftKind: 'trigger_nats',
		status: consumerStatus,
		forkConflictLabel: 'NATS trigger',
		modeError: (verb) => `Cannot ${verb} NATS trigger`,
		catchDelete: false,
		list: (a) => NatsTriggerService.listNatsTriggers(a),
		setMode: (a) => NatsTriggerService.setNatsTriggerMode(a),
		remove: (a) => NatsTriggerService.deleteNatsTrigger(a)
	},
	sqs: {
		title: 'SQS triggers',
		tooltip: 'SQS trigger',
		newLabel: 'SQS trigger',
		searchPlaceholder: 'Search SQS triggers',
		filterLabel: 'SQS trigger',
		empty: {
			title: 'No SQS triggers yet',
			description:
				'Windmill can consume messages from an SQS queue and trigger scripts or flows on each one.',
			actionLabel: 'Add an SQS trigger',
			aiId: 'sqs-triggers-empty-add',
			aiDescription: 'Add SQS trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'sqs',
		deployKind: 'sqs',
		shareKind: 'sqs_trigger',
		draftKind: 'trigger_sqs',
		status: {
			starting: 'SQS trigger is starting...',
			notConnected: 'Could not connect to SQS',
			disabled: 'Disabled because of an error',
			connected: 'Connected to SQS',
			shuttingDown: true,
			pings: false
		},
		forkConflictLabel: 'SQS trigger',
		modeError: (verb) => `Cannot ${verb} sqs trigger`,
		catchDelete: true,
		list: (a) => SqsTriggerService.listSqsTriggers(a),
		setMode: (a) => SqsTriggerService.setSqsTriggerMode(a),
		remove: (a) => SqsTriggerService.deleteSqsTrigger(a)
	},
	gcp: {
		title: 'GCP Pub/Sub triggers',
		tooltip: 'GCP Pub/Sub trigger',
		newLabel: 'GCP Pub/Sub trigger',
		searchPlaceholder: 'Search triggers',
		filterLabel: 'GCP Pub/Sub trigger',
		empty: {
			title: 'No GCP Pub/Sub triggers yet',
			description:
				'Windmill can subscribe to a GCP Pub/Sub topic and trigger scripts or flows on each message.',
			actionLabel: 'Add a GCP Pub/Sub trigger',
			aiId: 'gcp-triggers-empty-add',
			aiDescription: 'Add GCP Pub/Sub trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'gcp',
		deployKind: 'gcp',
		shareKind: 'gcp_trigger',
		draftKind: 'trigger_gcp',
		status: {
			starting: 'GCP Pub/Sub trigger is starting...',
			notConnected: 'Could not connect to GCP Pub/Sub',
			disabled: 'Disabled because of an error',
			connected: 'Connected to GCP Pub/Sub',
			shuttingDown: true,
			pings: false
		},
		forkConflictLabel: 'GCP Pub/Sub trigger',
		modeError: (verb) => `Cannot ${verb} GCP Pub/Sub trigger`,
		catchDelete: false,
		list: (a) => GcpTriggerService.listGcpTriggers(a),
		setMode: (a) => GcpTriggerService.setGcpTriggerMode(a),
		remove: (a) => GcpTriggerService.deleteGcpTrigger(a)
	},
	azure: {
		title: 'Azure Event Grid triggers',
		tooltip: 'Azure Event Grid trigger',
		newLabel: 'Azure Event Grid trigger',
		searchPlaceholder: 'Search triggers',
		filterLabel: 'Azure Event Grid trigger',
		empty: {
			title: 'No Azure Event Grid triggers yet',
			description:
				'Windmill can subscribe to an Azure Event Grid topic and trigger scripts or flows on each event.',
			actionLabel: 'Add an Azure Event Grid trigger',
			aiId: 'azure-triggers-empty-add',
			aiDescription: 'Add Azure Event Grid trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'azure',
		deployKind: 'azure',
		shareKind: 'azure_trigger',
		draftKind: 'trigger_azure',
		status: {
			starting: 'Azure Event Grid trigger is starting...',
			notConnected: 'Could not connect to Azure Event Grid',
			disabled: 'Disabled because of an error',
			connected: 'Connected to Azure Event Grid',
			shuttingDown: true,
			pings: false
		},
		forkConflictLabel: 'Azure trigger',
		modeError: (verb) => `Cannot ${verb} Azure Event Grid trigger`,
		catchDelete: false,
		list: (a) => AzureTriggerService.listAzureTriggers(a),
		setMode: (a) => AzureTriggerService.setAzureTriggerMode(a),
		remove: (a) => AzureTriggerService.deleteAzureTrigger(a)
	},
	mqtt: {
		title: 'MQTT triggers',
		tooltip: brokerTooltip('MQTT'),
		newLabel: 'MQTT trigger',
		searchPlaceholder: 'Search MQTT triggers',
		filterLabel: 'MQTT trigger',
		empty: {
			title: 'No MQTT triggers yet',
			description: brokerTooltip('MQTT'),
			actionLabel: 'Add an MQTT trigger',
			aiId: 'mqtt-triggers-empty-add',
			aiDescription: 'Add MQTT trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'mqtt',
		deployKind: 'mqtt',
		shareKind: 'mqtt_trigger',
		draftKind: 'trigger_mqtt',
		status: {
			starting: 'MQTT trigger is starting...',
			notConnected: 'MQTT trigger is not connected',
			disabled: 'MQTT trigger was disabled because of an error',
			connected: 'MQTT trigger is connected',
			shuttingDown: true,
			pings: true
		},
		forkConflictLabel: 'MQTT trigger',
		modeToastLabel: 'MQTT trigger',
		modeError: (verb) => `Cannot ${verb} mqtt trigger`,
		catchDelete: false,
		list: (a) => MqttTriggerService.listMqttTriggers(a),
		setMode: (a) => MqttTriggerService.setMqttTriggerMode(a),
		remove: (a) => MqttTriggerService.deleteMqttTrigger(a)
	},
	amqp: {
		title: 'AMQP triggers',
		tooltip: brokerTooltip('AMQP'),
		newLabel: 'AMQP trigger',
		searchPlaceholder: 'Search AMQP triggers',
		filterLabel: 'AMQP trigger',
		empty: {
			title: 'No AMQP triggers yet',
			description: brokerTooltip('AMQP'),
			actionLabel: 'Add an AMQP trigger',
			aiId: 'amqp-triggers-empty-add',
			aiDescription: 'Add AMQP trigger'
		},
		cloudDisabled: true,
		operatorGate: false,
		adminOnly: false,
		usedKind: 'amqp',
		deployKind: 'amqp',
		shareKind: 'amqp_trigger',
		draftKind: 'trigger_amqp',
		status: {
			starting: 'AMQP trigger is starting...',
			notConnected: 'AMQP trigger is not connected',
			disabled: 'AMQP trigger was disabled because of an error',
			connected: 'AMQP trigger is connected',
			shuttingDown: true,
			pings: true
		},
		forkConflictLabel: 'AMQP trigger',
		modeToastLabel: 'AMQP trigger',
		modeError: (verb) => `Cannot ${verb} amqp trigger`,
		catchDelete: false,
		list: (a) => AmqpTriggerService.listAmqpTriggers(a),
		setMode: (a) => AmqpTriggerService.setAmqpTriggerMode(a),
		remove: (a) => AmqpTriggerService.deleteAmqpTrigger(a)
	},
	email: {
		title: 'Custom email triggers',
		tooltip:
			'Every script and flow already has a canonical email trigger attached to it, this is to create additional parametrizable ones.',
		documentationLink: 'https://www.windmill.dev/docs/advanced/email_triggers',
		newLabel: 'email trigger',
		searchPlaceholder: 'Search email triggers',
		filterLabel: 'Email',
		empty: {
			title: 'No custom email triggers yet',
			description:
				'Every script and flow already has a canonical email trigger attached to it. These are additional parametrizable ones.',
			actionLabel: 'Add an email trigger',
			aiId: 'email-triggers-empty-add',
			aiDescription: 'Add email trigger'
		},
		cloudDisabled: false,
		operatorGate: true,
		adminOnly: true,
		usedKind: 'emails',
		deployKind: 'routes',
		shareKind: 'email_trigger',
		draftKind: 'trigger_email',
		modeError: (verb) => `Cannot ${verb} email trigger`,
		deleteToastLabel: 'Email trigger',
		catchDelete: true,
		list: (a) => EmailTriggerService.listEmailTriggers(a),
		setMode: (a) => EmailTriggerService.setEmailTriggerMode(a),
		remove: (a) => EmailTriggerService.deleteEmailTrigger(a)
	}
}
