import {
	AmqpTriggerService,
	ApiError,
	AzureTriggerService,
	EmailTriggerService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	SqsTriggerService,
	VariableService,
	WebsocketTriggerService
} from '$lib/gen'
import type { PageItemRef, TriggerKind } from './previewPaths'

type Get = (args: { workspace: string; path: string; getDraft: boolean }) => Promise<unknown>

const TRIGGER_GETS: Record<TriggerKind, Get> = {
	http: (a) => HttpTriggerService.getHttpTrigger(a),
	websocket: (a) => WebsocketTriggerService.getWebsocketTrigger(a),
	postgres: (a) => PostgresTriggerService.getPostgresTrigger(a),
	kafka: (a) => KafkaTriggerService.getKafkaTrigger(a),
	nats: (a) => NatsTriggerService.getNatsTrigger(a),
	mqtt: (a) => MqttTriggerService.getMqttTrigger(a),
	amqp: (a) => AmqpTriggerService.getAmqpTrigger(a),
	sqs: (a) => SqsTriggerService.getSqsTrigger(a),
	gcp: (a) => GcpTriggerService.getGcpTrigger(a),
	azure: (a) => AzureTriggerService.getAzureTrigger(a),
	email: (a) => EmailTriggerService.getEmailTrigger(a)
}

function getFor(ref: PageItemRef): Get {
	switch (ref.kind) {
		case 'variable':
			return (a) => VariableService.getVariable(a)
		case 'resource':
			return (a) => ResourceService.getResource(a)
		case 'schedule':
			return (a) => ScheduleService.getSchedule(a)
		case 'trigger':
			return TRIGGER_GETS[ref.triggerKind]
	}
}

/** A page item as its editor would load it — deployed, or only a draft — or undefined when it
 * is neither: deleted, or a draft that was discarded. Any other failure is thrown, for the
 * caller to leave to the editor rather than report as a missing item. */
export async function lookupPageItem(
	ref: PageItemRef,
	workspace: string
): Promise<Record<string, any> | undefined> {
	try {
		return (await getFor(ref)({ workspace, path: ref.path, getDraft: true })) as Record<string, any>
	} catch (e) {
		if (e instanceof ApiError && e.status === 404) return undefined
		throw e
	}
}
