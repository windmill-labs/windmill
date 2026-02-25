import { minimatch } from 'minimatch'
import {
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	SqsTriggerService,
	WebsocketTriggerService,
	type GcpTriggerData,
	type WorkspaceDeployUISettings
} from './gen'
import type { TriggerKind } from './components/triggers'
import { base } from './base'

type DeployUIType = 'script' | 'flow' | 'app' | 'resource' | 'variable' | 'secret' | 'trigger'

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
	| 'trigger'

export const ALL_DEPLOYABLE: WorkspaceDeployUISettings = {
	include_path: [],
	include_type: ['script', 'flow', 'app', 'resource', 'variable', 'secret', 'trigger']
}

export type AdditionalInformation = {
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
	} else if (triggerKind === 'gcp') {
		return await GcpTriggerService.existsGcpTrigger(data)
	} else if (triggerKind === 'websockets') {
		return await WebsocketTriggerService.existsWebsocketTrigger(data)
	} else if (triggerKind === 'nats') {
		return await NatsTriggerService.existsNatsTrigger(data)
	} else if (triggerKind === 'schedules') {
		return await ScheduleService.existsSchedule(data)
	}

	throw new Error(
		`Unexpected trigger kind ${triggerKind}. Allowed kinds are: routes, kafka, mqtt, postgres, sqs, gcp, websockets, nats, schedules.`
	)
}

/**
 * Get trigger deployment data with optional email preservation.
 * @param onBehalfOfEmail - If set, the trigger will be deployed with this email and preserve_email=true.
 */
export async function getTriggersDeployData(
	kind: TriggerKind,
	path: string,
	workspace: string,
	onBehalfOfEmail?: string
) {
	const preserveEmail = onBehalfOfEmail !== undefined

	if (kind === 'sqs') {
		const sqsTrigger = await SqsTriggerService.getSqsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...sqsTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: SqsTriggerService.createSqsTrigger,
			updateFn: SqsTriggerService.updateSqsTrigger
		}
	} else if (kind === 'kafka') {
		const kafkaTrigger = await KafkaTriggerService.getKafkaTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...kafkaTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: KafkaTriggerService.createKafkaTrigger,
			updateFn: KafkaTriggerService.updateKafkaTrigger
		}
	} else if (kind === 'mqtt') {
		const mqttTrigger = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...mqttTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: MqttTriggerService.createMqttTrigger,
			updateFn: MqttTriggerService.updateMqttTrigger
		}
	} else if (kind === 'nats') {
		const natsTrigger = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...natsTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: NatsTriggerService.createNatsTrigger,
			updateFn: NatsTriggerService.updateNatsTrigger
		}
	} else if (kind === 'gcp') {
		const gcpTrigger = await GcpTriggerService.getGcpTrigger({
			workspace: workspace!,
			path: path
		})

		gcpTrigger.subscription_id = ''
		gcpTrigger.subscription_mode = 'create_update'

		if (gcpTrigger.delivery_config) {
			gcpTrigger.delivery_config.audience = ''
		}

		const data: GcpTriggerData = {
			...gcpTrigger,
			delivery_config: gcpTrigger.delivery_config ?? undefined,
			base_endpoint:
				gcpTrigger.delivery_type === 'push' ? `${window.location.origin}${base}` : undefined,
			email: onBehalfOfEmail,
			preserve_email: preserveEmail
		}

		return {
			data,
			createFn: GcpTriggerService.createGcpTrigger,
			updateFn: GcpTriggerService.updateGcpTrigger
		}
	} else if (kind === 'postgres') {
		const postgresTrigger = await PostgresTriggerService.getPostgresTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...postgresTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: PostgresTriggerService.createPostgresTrigger,
			updateFn: PostgresTriggerService.updatePostgresTrigger
		}
	} else if (kind === 'websockets') {
		const websocketTrigger = await WebsocketTriggerService.getWebsocketTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...websocketTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: WebsocketTriggerService.createWebsocketTrigger,
			updateFn: WebsocketTriggerService.updateWebsocketTrigger
		}
	} else if (kind === 'routes') {
		const httpTrigger = await HttpTriggerService.getHttpTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: { ...httpTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: HttpTriggerService.createHttpTrigger,
			updateFn: HttpTriggerService.updateHttpTrigger
		}
	} else if (kind === 'schedules') {
		const schedulesTrigger = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})
		return {
			data: { ...schedulesTrigger, email: onBehalfOfEmail, preserve_email: preserveEmail },
			createFn: ScheduleService.createSchedule,
			updateFn: ScheduleService.updateSchedule
		}
	}

	throw new Error(`Unexpected trigger kind got: ${kind}`)
}

export async function getTriggerValue(kind: TriggerKind, path: string, workspace: string) {
	if (kind === 'sqs') {
		const { mode, script_path, is_flow, queue_url, aws_resource_path, message_attributes } =
			await SqsTriggerService.getSqsTrigger({
				workspace: workspace!,
				path: path
			})

		return {
			mode,
			script_path,
			is_flow,
			queue_url,
			aws_resource_path,
			message_attributes
		}
	} else if (kind === 'kafka') {
		const { mode, script_path, is_flow, kafka_resource_path, topics, group_id } =
			await KafkaTriggerService.getKafkaTrigger({
				workspace: workspace!,
				path: path
			})

		return {
			mode,
			script_path,
			is_flow,
			kafka_resource_path,
			topics,
			group_id
		}
	} else if (kind === 'mqtt') {
		const {
			mode,
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
			mode,
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
			mode,
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
			mode,
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
			mode,
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
			mode,
			script_path,
			is_flow,
			postgres_resource_path,
			replication_slot_name,
			publication_name
		}
	} else if (kind === 'gcp') {
		const { mode, script_path, is_flow, gcp_resource_path } = await GcpTriggerService.getGcpTrigger(
			{
				workspace: workspace!,
				path: path
			}
		)

		return {
			mode,
			script_path,
			is_flow,
			gcp_resource_path
		}
	} else if (kind === 'websockets') {
		const {
			mode,
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
			mode,
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
			request_type,
			authentication_method,
			is_static_website,
			authentication_resource_path
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
			request_type,
			authentication_method,
			is_static_website,
			authentication_resource_path
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

/**
 * Get the email for a trigger (used for on_behalf_of during deployment).
 */
export async function getTriggerEmail(
	kind: TriggerKind,
	path: string,
	workspace: string
): Promise<string | undefined> {
	try {
		if (kind === 'sqs') {
			const trigger = await SqsTriggerService.getSqsTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'kafka') {
			const trigger = await KafkaTriggerService.getKafkaTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'mqtt') {
			const trigger = await MqttTriggerService.getMqttTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'nats') {
			const trigger = await NatsTriggerService.getNatsTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'gcp') {
			const trigger = await GcpTriggerService.getGcpTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'postgres') {
			const trigger = await PostgresTriggerService.getPostgresTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'websockets') {
			const trigger = await WebsocketTriggerService.getWebsocketTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'routes') {
			const trigger = await HttpTriggerService.getHttpTrigger({ workspace, path })
			return trigger.edited_by
		} else if (kind === 'schedules') {
			const trigger = await ScheduleService.getSchedule({ workspace, path })
			return trigger.edited_by
		}
	} catch {
		// Trigger may not exist in the workspace
	}
	return undefined
}

function retrieveScriptOrFlowKind(path: string, is_flow: boolean): { kind: Kind; path: string } {
	return {
		kind: is_flow ? 'flow' : 'script',
		path
	}
}

function retrieveKindsValues({
	resource_path,
	script_path,
	is_flow
}: {
	resource_path?: string
	script_path: string
	is_flow: boolean
}) {
	const result: { kind: Kind; path: string }[] = []

	if (resource_path) {
		result.push({ kind: 'resource', path: resource_path })
	}
	result.push(retrieveScriptOrFlowKind(script_path, is_flow))
	return result
}

export async function getTriggerDependency(kind: TriggerKind, path: string, workspace: string) {
	let result: { kind: Kind; path: string }[]
	if (kind === 'sqs') {
		const { aws_resource_path, script_path, is_flow } = await SqsTriggerService.getSqsTrigger({
			workspace: workspace!,
			path: path
		})

		result = retrieveKindsValues({ resource_path: aws_resource_path, script_path, is_flow })
	} else if (kind === 'kafka') {
		const { kafka_resource_path, script_path, is_flow } = await KafkaTriggerService.getKafkaTrigger(
			{
				workspace: workspace!,
				path: path
			}
		)

		result = retrieveKindsValues({ resource_path: kafka_resource_path, script_path, is_flow })
	} else if (kind === 'mqtt') {
		const { mqtt_resource_path, script_path, is_flow } = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		result = retrieveKindsValues({ resource_path: mqtt_resource_path, script_path, is_flow })
	} else if (kind === 'nats') {
		const { nats_resource_path, script_path, is_flow } = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		result = retrieveKindsValues({ resource_path: nats_resource_path, script_path, is_flow })
	} else if (kind === 'postgres') {
		const { postgres_resource_path, script_path, is_flow } =
			await PostgresTriggerService.getPostgresTrigger({
				workspace: workspace!,
				path: path
			})

		result = retrieveKindsValues({ resource_path: postgres_resource_path, script_path, is_flow })
	} else if (kind === 'gcp') {
		const { gcp_resource_path, script_path, is_flow } = await GcpTriggerService.getGcpTrigger({
			workspace: workspace!,
			path: path
		})

		result = retrieveKindsValues({ resource_path: gcp_resource_path, script_path, is_flow })
	} else if (kind === 'websockets') {
		const { script_path, is_flow, url, initial_messages } =
			await WebsocketTriggerService.getWebsocketTrigger({
				workspace: workspace!,
				path: path
			})

		result = retrieveKindsValues({ script_path, is_flow })

		const SCRIPT_PREFIX = '$script:'
		const FLOW_PREFIX = '$flow:'

		if (url.startsWith(SCRIPT_PREFIX))
			result.push(retrieveScriptOrFlowKind(url.substring(SCRIPT_PREFIX.length), false))
		else if (url.startsWith(FLOW_PREFIX))
			result.push(retrieveScriptOrFlowKind(url.substring(FLOW_PREFIX.length), true))

		initial_messages?.map((message) => {
			if ('runnable_result' in message) {
				result.push(
					retrieveScriptOrFlowKind(message.runnable_result.path, message.runnable_result.is_flow)
				)
			}
		})
	} else if (kind === 'routes') {
		const { script_path, is_flow, authentication_resource_path } =
			await HttpTriggerService.getHttpTrigger({
				workspace: workspace!,
				path: path
			})

		result = retrieveKindsValues({
			script_path,
			is_flow,
			resource_path: authentication_resource_path ?? undefined
		})
	} else if (kind === 'schedules') {
		const { script_path, is_flow } = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})

		result = retrieveKindsValues({ script_path, is_flow })
	} else {
		throw new Error(`Unexpected trigger kind got: ${kind}`)
	}
	return result
}
