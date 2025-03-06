import { minimatch } from 'minimatch'
import {
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	SqsTriggerService,
	WebsocketTriggerService,
	type WorkspaceDeployUISettings
} from './gen'
import type { TriggerKind } from './components/triggers'

type DeployUIType = 'script' | 'flow' | 'app' | 'resource' | 'variable' | 'secret' | 'triggers'

export type Kind =
	| 'script'
	| 'resource'
	| 'schedule'
	| 'variable'
	| 'flow'
	| 'app'
	| 'raw_app'
	| 'resource_type'
	| 'folder'
	| 'triggers'

export const ALL_DEPLOYABLE: WorkspaceDeployUISettings = {
	include_path: [],
	include_type: ['script', 'flow', 'app', 'resource', 'variable', 'secret', 'triggers']
}

export type AdditionalInformations = {
	triggers?: {
		kind: TriggerKind
	}
}

export function isDeployable(
	type: DeployUIType,
	path: string,
	deployUiSettings: WorkspaceDeployUISettings | undefined
) {
	if (deployUiSettings == undefined) {
		return false
	}

	if (deployUiSettings.include_type != undefined && !deployUiSettings.include_type.includes(type)) {
		return false
	}

	if (
		deployUiSettings.include_path != undefined &&
		deployUiSettings.include_path.length != 0 &&
		deployUiSettings.include_path.every((x) => !minimatch(path, x))
	) {
		return false
	}

	return true
}

export async function existsTrigger(
	data: { workspace: string; path: string },
	triggerKind: TriggerKind
) {
	if (triggerKind === 'routes') {
		return await HttpTriggerService.existsHttpTrigger(data)
	} else if (triggerKind === 'kafka') {
		return await KafkaTriggerService.existsKafkaTrigger(data)
	} else if (triggerKind === 'mqtt') {
		return await MqttTriggerService.existsMqttTrigger(data)
	} else if (triggerKind === 'postgres') {
		return await PostgresTriggerService.existsPostgresTrigger(data)
	} else if (triggerKind === 'sqs') {
		return await SqsTriggerService.existsSqsTrigger(data)
	} else if (triggerKind === 'websockets') {
		return await WebsocketTriggerService.existsWebsocketTrigger(data)
	} else if (triggerKind === 'nats') {
		return await NatsTriggerService.existsNatsTrigger(data)
	} else if (triggerKind === 'schedules') {
		return await ScheduleService.existsSchedule(data)
	}

	throw new Error(`Unexpected trigger kind ${triggerKind}`)
}

export async function getTriggersDeployData(kind: TriggerKind, path: string, workspace: string) {
	if (kind === 'sqs') {
		const sqsTrigger = await SqsTriggerService.getSqsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: sqsTrigger,
			createFn: SqsTriggerService.createSqsTrigger,
			updateFn: SqsTriggerService.updateSqsTrigger
		}
	} else if (kind === 'kafka') {
		const kafkaTrigger = await KafkaTriggerService.getKafkaTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: kafkaTrigger,
			createFn: KafkaTriggerService.createKafkaTrigger,
			updateFn: KafkaTriggerService.updateKafkaTrigger
		}
	} else if (kind === 'mqtt') {
		const mqttTrigger = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: mqttTrigger,
			createFn: MqttTriggerService.createMqttTrigger,
			updateFn: MqttTriggerService.updateMqttTrigger
		}
	} else if (kind === 'nats') {
		const natsTrigger = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: natsTrigger,
			createFn: NatsTriggerService.createNatsTrigger,
			updateFn: NatsTriggerService.updateNatsTrigger
		}
	} else if (kind === 'postgres') {
		const postgresTrigger = await PostgresTriggerService.getPostgresTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: postgresTrigger,
			createFn: PostgresTriggerService.createPostgresTrigger,
			updateFn: PostgresTriggerService.updatePostgresTrigger
		}
	} else if (kind === 'websockets') {
		const websocketTrigger = await WebsocketTriggerService.getWebsocketTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: websocketTrigger,
			createFn: WebsocketTriggerService.createWebsocketTrigger,
			updateFn: WebsocketTriggerService.updateWebsocketTrigger
		}
	} else if (kind === 'routes') {
		const httpTrigger = await HttpTriggerService.getHttpTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: httpTrigger,
			createFn: HttpTriggerService.createHttpTrigger,
			updateFn: HttpTriggerService.updateHttpTrigger
		}
	} else if (kind === 'schedules') {
		const schedulesTrigger = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})
		return {
			data: schedulesTrigger,
			createFn: ScheduleService.createSchedule,
			updateFn: ScheduleService.updateSchedule
		}
	}

	throw new Error(`Unexpected trigger kind got: ${kind}`)
}

export async function getTriggerValue(kind: TriggerKind, path: string, workspace: string) {
	if (kind === 'sqs') {
		const { enabled, script_path, is_flow, queue_url, aws_resource_path, message_attributes } =
			await SqsTriggerService.getSqsTrigger({
				workspace: workspace!,
				path: path
			})

		return {
			enabled,
			script_path,
			is_flow,
			queue_url,
			aws_resource_path,
			message_attributes
		}
	} else if (kind === 'kafka') {
		const { enabled, script_path, is_flow, kafka_resource_path, topics, group_id } =
			await KafkaTriggerService.getKafkaTrigger({
				workspace: workspace!,
				path: path
			})

		return {
			enabled,
			script_path,
			is_flow,
			kafka_resource_path,
			topics,
			group_id
		}
	} else if (kind === 'mqtt') {
		const {
			enabled,
			script_path,
			is_flow,
			mqtt_resource_path,
			v3_config,
			v5_config,
			client_id,
			subscribe_topics,
			client_version
		} = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			enabled,
			script_path,
			is_flow,
			mqtt_resource_path,
			v3_config,
			v5_config,
			client_id,
			subscribe_topics,
			client_version
		}
	} else if (kind === 'nats') {
		const {
			enabled,
			script_path,
			is_flow,
			nats_resource_path,
			use_jetstream,
			stream_name,
			consumer_name,
			subjects
		} = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			enabled,
			script_path,
			is_flow,
			nats_resource_path,
			use_jetstream,
			stream_name,
			consumer_name,
			subjects
		}
	} else if (kind === 'postgres') {
		const {
			enabled,
			script_path,
			is_flow,
			postgres_resource_path,
			replication_slot_name,
			publication_name
		} = await PostgresTriggerService.getPostgresTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			enabled,
			script_path,
			is_flow,
			postgres_resource_path,
			replication_slot_name,
			publication_name
		}
	} else if (kind === 'websockets') {
		const {
			enabled,
			script_path,
			is_flow,
			url,
			url_runnable_args,
			can_return_message,
			filters,
			initial_messages
		} = await WebsocketTriggerService.getWebsocketTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			enabled,
			script_path,
			is_flow,
			url,
			url_runnable_args,
			can_return_message,
			filters,
			initial_messages
		}
	} else if (kind === 'routes') {
		const {
			script_path,
			is_flow,
			http_method,
			route_path,
			static_asset_config,
			is_async,
			requires_auth,
			is_static_website
		} = await HttpTriggerService.getHttpTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			script_path,
			is_flow,
			http_method,
			route_path,
			static_asset_config,
			is_async,
			requires_auth,
			is_static_website
		}
	} else if (kind === 'schedules') {
		const {
			script_path,
			is_flow,
			on_failure,
			schedule,
			timezone,
			on_failure_times,
			on_failure_exact,
			on_failure_extra_args,
			on_recovery,
			on_recovery_times,
			on_recovery_extra_args,
			on_success,
			on_success_extra_args,
			ws_error_handler_muted,
			retry,
			summary,
			no_flow_overlap,
			tag,
			paused_until,
			cron_version
		} = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})

		return {
			script_path,
			is_flow,
			on_failure,
			schedule,
			timezone,
			on_failure_times,
			on_failure_exact,
			on_failure_extra_args,
			on_recovery,
			on_recovery_times,
			on_recovery_extra_args,
			on_success,
			on_success_extra_args,
			ws_error_handler_muted,
			retry,
			summary,
			no_flow_overlap,
			tag,
			paused_until,
			cron_version
		}
	}

	throw new Error(`Unexpected trigger kind got: ${kind}`)
}

function getKindValue(script_path: string, is_flow: boolean): { kind: Kind; path: string } {
	return { kind: is_flow ? 'flow' : 'script', path: script_path }
}

export async function getTriggerDependency(kind: TriggerKind, path: string, workspace: string) {
	const result: { kind: Kind; path: string }[] = []
	if (kind === 'sqs') {
		const { aws_resource_path, script_path, is_flow } = await SqsTriggerService.getSqsTrigger({
			workspace: workspace!,
			path: path
		})

		result.push({ kind: 'resource', path: aws_resource_path })
		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'kafka') {
		const { kafka_resource_path, script_path, is_flow } = await KafkaTriggerService.getKafkaTrigger(
			{
				workspace: workspace!,
				path: path
			}
		)

		result.push({ kind: 'resource', path: kafka_resource_path })
		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'mqtt') {
		const { mqtt_resource_path, script_path, is_flow } = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		result.push({ kind: 'resource', path: mqtt_resource_path })
		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'nats') {
		const { nats_resource_path, script_path, is_flow } = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		result.push({ kind: 'resource', path: nats_resource_path })
		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'postgres') {
		const { postgres_resource_path, script_path, is_flow } =
			await PostgresTriggerService.getPostgresTrigger({
				workspace: workspace!,
				path: path
			})

		result.push({ kind: 'resource', path: postgres_resource_path })
		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'websockets') {
		const { script_path, is_flow } = await WebsocketTriggerService.getWebsocketTrigger({
			workspace: workspace!,
			path: path
		})

		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'routes') {
		const { script_path, is_flow } = await HttpTriggerService.getHttpTrigger({
			workspace: workspace!,
			path: path
		})

		result.push(getKindValue(script_path, is_flow))
	} else if (kind === 'schedules') {
		const { script_path, is_flow } = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})

		result.push(getKindValue(script_path, is_flow))
	} else {
		throw new Error(`Unexpected trigger kind got: ${kind}`)
	}
	return result
}
