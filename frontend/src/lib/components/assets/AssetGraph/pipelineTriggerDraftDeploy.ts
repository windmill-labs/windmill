import { usedTriggerKinds } from '$lib/stores'
import { formatCron } from '$lib/utils'
import { saveScheduleFromCfg } from '$lib/components/flows/scheduleUtils'
import { saveEmailTriggerFromCfg } from '$lib/components/triggers/email/utils'
import { saveKafkaTriggerFromCfg } from '$lib/components/triggers/kafka/utils'
import { saveMqttTriggerFromCfg } from '$lib/components/triggers/mqtt/utils'
import { saveAmqpTriggerFromCfg } from '$lib/components/triggers/amqp/utils'
import { saveNatsTriggerFromCfg } from '$lib/components/triggers/nats/utils'
import { savePostgresTriggerFromCfg } from '$lib/components/triggers/postgres/utils'
import { saveSqsTriggerFromCfg } from '$lib/components/triggers/sqs/utils'
import { saveGcpTriggerFromCfg } from '$lib/components/triggers/gcp/utils'
import {
	AmqpTriggerService,
	EmailTriggerService,
	GcpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	SqsTriggerService
} from '$lib/gen'
import type { NativeTriggerKind, PipelineTriggerDraft } from './types'

/** Creates the trigger through the same helper its editor saves with; that helper
 * reports the outcome as a toast and resolves to whether it succeeded. */
export function deployTriggerDraft(d: PipelineTriggerDraft, workspace: string): Promise<boolean> {
	const cfg = d.config
	switch (d.kind) {
		case 'schedule':
			return saveScheduleFromCfg({ ...cfg, schedule: formatCron(cfg.schedule) }, false, workspace)
		case 'email':
			return saveEmailTriggerFromCfg('', cfg, false, workspace, false, usedTriggerKinds)
		case 'kafka':
			return saveKafkaTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'mqtt':
			return saveMqttTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'amqp':
			return saveAmqpTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'nats':
			return saveNatsTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'postgres':
			return savePostgresTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'sqs':
			return saveSqsTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
		case 'gcp':
			return saveGcpTriggerFromCfg('', cfg, false, workspace, usedTriggerKinds)
	}
}

/** Deletes a deployed trigger row. False for the kinds that have none (webhook,
 * data upload). */
export async function deleteTriggerRow(
	kind: NativeTriggerKind,
	path: string,
	workspace: string
): Promise<boolean> {
	switch (kind) {
		case 'schedule':
			await ScheduleService.deleteSchedule({ workspace, path })
			return true
		case 'kafka':
			await KafkaTriggerService.deleteKafkaTrigger({ workspace, path })
			return true
		case 'mqtt':
			await MqttTriggerService.deleteMqttTrigger({ workspace, path })
			return true
		case 'amqp':
			await AmqpTriggerService.deleteAmqpTrigger({ workspace, path })
			return true
		case 'nats':
			await NatsTriggerService.deleteNatsTrigger({ workspace, path })
			return true
		case 'postgres':
			await PostgresTriggerService.deletePostgresTrigger({ workspace, path })
			return true
		case 'sqs':
			await SqsTriggerService.deleteSqsTrigger({ workspace, path })
			return true
		case 'gcp':
			await GcpTriggerService.deleteGcpTrigger({ workspace, path })
			return true
		case 'email':
			await EmailTriggerService.deleteEmailTrigger({ workspace, path })
			return true
		default:
			return false
	}
}
