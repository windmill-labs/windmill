import { minimatch } from 'minimatch'
import {
	AzureTriggerService,
	EmailTriggerService,
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
	// Per-kind trigger names returned by the backend's `compareWorkspaces` API.
	| 'http_trigger'
	| 'websocket_trigger'
	| 'kafka_trigger'
	| 'nats_trigger'
	| 'postgres_trigger'
	| 'mqtt_trigger'
	| 'sqs_trigger'
	| 'gcp_trigger'
	| 'azure_trigger'
	| 'email_trigger'
	// Legacy generic kind used by the cross-workspace `DeployWorkspace` UI,
	// which carries the trigger sub-kind in `additionalInformation`.
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
	} else if (triggerKind === 'azure') {
		return await AzureTriggerService.existsAzureTrigger(data)
	} else if (triggerKind === 'emails') {
		return await EmailTriggerService.existsEmailTrigger(data)
	} else if (triggerKind === 'schedules') {
		return await ScheduleService.existsSchedule(data)
	}

	throw new Error(
		`Unexpected trigger kind ${triggerKind}. Allowed kinds are: routes, kafka, mqtt, postgres, sqs, gcp, websockets, nats, azure, emails, schedules.`
	)
}

/**
 * Strip operational state (`mode`, `enabled`) from a trigger/schedule payload
 * before sending it to an update endpoint via the merge UI. The backend's
 * `update_trigger` handler preserves the target row's existing `mode` when
 * both fields are absent from the request (`is_mode_unspecified()`), so
 * stripping here lets a fork→parent (or parent→fork) deploy carry config
 * changes without flipping the target's enabled/disabled state. Schedules'
 * `EditSchedule` already lacks `enabled` on the backend, but stripping keeps
 * the intent explicit and matches the YAML/CLI round-trip behavior.
 *
 * Used by the legacy `kind === 'trigger'` path in `utils_workspace_deploy.ts`
 * (the cross-workspace deploy UI). The merge-UI deploy goes through the
 * shared `deployItem` in `windmill-utils-internal`, which applies its own
 * `stripOperationalStateOnUpdate` at the dispatch layer.
 */
export function stripOperationalState<T extends Record<string, any>>(
	payload: T
): Omit<T, 'mode' | 'enabled'> {
	const { mode: _mode, enabled: _enabled, ...rest } = payload
	return rest
}

/**
 * Get trigger deployment data with optional permissioned_as preservation.
 * @param onBehalfOf - If set, the trigger will be deployed with this permissioned_as (u/username or g/group) and preserve_permissioned_as=true.
 */
export async function getTriggersDeployData(
	kind: TriggerKind,
	path: string,
	workspace: string,
	onBehalfOf?: string
) {
	const preservePermissionedAs = onBehalfOf !== undefined

	if (kind === 'sqs') {
		const sqsTrigger = await SqsTriggerService.getSqsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...sqsTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: SqsTriggerService.createSqsTrigger,
			updateFn: SqsTriggerService.updateSqsTrigger
		}
	} else if (kind === 'kafka') {
		const kafkaTrigger = await KafkaTriggerService.getKafkaTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...kafkaTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: KafkaTriggerService.createKafkaTrigger,
			updateFn: KafkaTriggerService.updateKafkaTrigger
		}
	} else if (kind === 'mqtt') {
		const mqttTrigger = await MqttTriggerService.getMqttTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...mqttTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: MqttTriggerService.createMqttTrigger,
			updateFn: MqttTriggerService.updateMqttTrigger
		}
	} else if (kind === 'nats') {
		const natsTrigger = await NatsTriggerService.getNatsTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...natsTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
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
			permissioned_as: onBehalfOf,
			preserve_permissioned_as: preservePermissionedAs
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
			data: {
				...postgresTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: PostgresTriggerService.createPostgresTrigger,
			updateFn: PostgresTriggerService.updatePostgresTrigger
		}
	} else if (kind === 'websockets') {
		const websocketTrigger = await WebsocketTriggerService.getWebsocketTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...websocketTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: WebsocketTriggerService.createWebsocketTrigger,
			updateFn: WebsocketTriggerService.updateWebsocketTrigger
		}
	} else if (kind === 'routes') {
		const httpTrigger = await HttpTriggerService.getHttpTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...httpTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: HttpTriggerService.createHttpTrigger,
			updateFn: HttpTriggerService.updateHttpTrigger
		}
	} else if (kind === 'azure') {
		const azureTrigger = await AzureTriggerService.getAzureTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...azureTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: AzureTriggerService.createAzureTrigger,
			updateFn: AzureTriggerService.updateAzureTrigger
		}
	} else if (kind === 'emails') {
		const emailTrigger = await EmailTriggerService.getEmailTrigger({
			workspace: workspace!,
			path: path
		})

		return {
			data: {
				...emailTrigger,
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: EmailTriggerService.createEmailTrigger,
			updateFn: EmailTriggerService.updateEmailTrigger
		}
	} else if (kind === 'schedules') {
		const schedulesTrigger = await ScheduleService.getSchedule({
			workspace: workspace!,
			path: path
		})
		return {
			data: {
				...schedulesTrigger,
				// permissioned_as is only set on create, not update
				permissioned_as: onBehalfOf,
				preserve_permissioned_as: preservePermissionedAs
			},
			createFn: ScheduleService.createSchedule,
			updateFn: ScheduleService.updateSchedule
		}
	}

	throw new Error(`Unexpected trigger kind got: ${kind}`)
}

/**
 * Runtime fields stripped from the trigger/schedule diff so the drawer mirrors
 * the backend's `compare_two_trigger_or_schedule` semantics — same set as
 * `TRIGGER_COMPARE_IGNORE` and `stripTriggerOrScheduleRuntimeFields` in the
 * shared deploy module.
 */
const TRIGGER_RUNTIME_IGNORE = new Set([
	'workspace_id',
	'edited_by',
	'edited_at',
	'email',
	'error',
	'enabled',
	'mode',
	'server_id',
	'last_server_ping',
	'extra_perms',
	'permissioned_as',
	// Server-managed (kept in sync with backend `TRIGGER_COMPARE_IGNORE`).
	'subscription_id',
	'push_auth_config'
])

function stripTriggerRuntimeFields<T extends Record<string, any>>(row: T): Partial<T> {
	const out: Record<string, any> = {}
	for (const [k, v] of Object.entries(row)) {
		if (!TRIGGER_RUNTIME_IGNORE.has(k)) out[k] = v
	}
	return out as Partial<T>
}

export async function getTriggerValue(kind: TriggerKind, path: string, workspace: string) {
	let trigger: Record<string, any>
	if (kind === 'sqs') {
		trigger = await SqsTriggerService.getSqsTrigger({ workspace, path })
	} else if (kind === 'kafka') {
		trigger = await KafkaTriggerService.getKafkaTrigger({ workspace, path })
	} else if (kind === 'mqtt') {
		trigger = await MqttTriggerService.getMqttTrigger({ workspace, path })
	} else if (kind === 'nats') {
		trigger = await NatsTriggerService.getNatsTrigger({ workspace, path })
	} else if (kind === 'postgres') {
		trigger = await PostgresTriggerService.getPostgresTrigger({ workspace, path })
	} else if (kind === 'gcp') {
		trigger = await GcpTriggerService.getGcpTrigger({ workspace, path })
	} else if (kind === 'websockets') {
		trigger = await WebsocketTriggerService.getWebsocketTrigger({ workspace, path })
	} else if (kind === 'routes') {
		trigger = await HttpTriggerService.getHttpTrigger({ workspace, path })
	} else if (kind === 'schedules') {
		trigger = await ScheduleService.getSchedule({ workspace, path })
	} else if (kind === 'azure') {
		trigger = await AzureTriggerService.getAzureTrigger({ workspace, path })
	} else if (kind === 'emails') {
		trigger = await EmailTriggerService.getEmailTrigger({ workspace, path })
	} else {
		throw new Error(`Unexpected trigger kind got: ${kind}`)
	}
	return stripTriggerRuntimeFields(trigger)
}

/**
 * Get the permissioned_as for a trigger (used for on_behalf_of during deployment).
 */
export async function getTriggerPermissionedAs(
	kind: TriggerKind,
	path: string,
	workspace: string
): Promise<string | undefined> {
	try {
		if (kind === 'sqs') {
			const trigger = await SqsTriggerService.getSqsTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'kafka') {
			const trigger = await KafkaTriggerService.getKafkaTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'mqtt') {
			const trigger = await MqttTriggerService.getMqttTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'nats') {
			const trigger = await NatsTriggerService.getNatsTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'gcp') {
			const trigger = await GcpTriggerService.getGcpTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'postgres') {
			const trigger = await PostgresTriggerService.getPostgresTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'websockets') {
			const trigger = await WebsocketTriggerService.getWebsocketTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'routes') {
			const trigger = await HttpTriggerService.getHttpTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'schedules') {
			const trigger = await ScheduleService.getSchedule({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'azure') {
			const trigger = await AzureTriggerService.getAzureTrigger({ workspace, path })
			return trigger.permissioned_as
		} else if (kind === 'emails') {
			const trigger = await EmailTriggerService.getEmailTrigger({ workspace, path })
			return trigger.permissioned_as
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
