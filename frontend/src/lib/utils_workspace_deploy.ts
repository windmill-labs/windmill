import {
	AppService,
	FlowService,
	FolderService,
	ResourceService,
	ScheduleService,
	ScriptService,
	VariableService
} from '$lib/gen'
import { getAllModules } from './components/flows/flowExplorer'
import {
	existsTrigger,
	getTriggersDeployData,
	getTriggerEmail,
	getTriggerValue,
	type AdditionalInformation,
	type Kind
} from '$lib/utils_deployable'
import type { TriggerKind } from './components/triggers'

export interface DeployItemParams {
	kind: Kind
	path: string
	workspaceFrom: string
	workspaceTo: string
	additionalInformation?: AdditionalInformation
	/** The email to use for on_behalf_of. If set, preserve_on_behalf_of will be true. If undefined, deploying user's email is used. */
	onBehalfOfEmail?: string
}

export interface DeployResult {
	success: boolean
	error?: string
}

/**
 * Deploy an item from one workspace to another.
 * Handles all item kinds: flow, script, app, variable, resource, resource_type, folder, trigger.
 */
export async function deployItem(params: DeployItemParams): Promise<DeployResult> {
	const { kind, path, workspaceFrom, workspaceTo, additionalInformation, onBehalfOfEmail } = params
	// When onBehalfOfEmail is set, we preserve the on_behalf_of setting with the specified email
	const preserveOnBehalfOf = onBehalfOfEmail !== undefined

	try {
		const alreadyExists = await checkItemExists(kind, path, workspaceTo, additionalInformation)

		if (kind === 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace: workspaceFrom,
				path: path
			})
			getAllModules(flow.value.modules).forEach((x) => {
				if (x.value.type === 'script' && x.value.hash != undefined) {
					x.value.hash = undefined
				}
			})
			if (alreadyExists) {
				await FlowService.updateFlow({
					workspace: workspaceTo,
					path: path,
					requestBody: {
						...flow,
						preserve_on_behalf_of: preserveOnBehalfOf,
						on_behalf_of_email: onBehalfOfEmail
					}
				})
			} else {
				await FlowService.createFlow({
					workspace: workspaceTo,
					requestBody: {
						...flow,
						preserve_on_behalf_of: preserveOnBehalfOf,
						on_behalf_of_email: onBehalfOfEmail
					}
				})
			}
		} else if (kind === 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace: workspaceFrom,
				path: path
			})
			await ScriptService.createScript({
				workspace: workspaceTo,
				requestBody: {
					...script,
					lock: script.lock,
					parent_hash: alreadyExists
						? (
								await ScriptService.getScriptByPath({
									workspace: workspaceTo,
									path: path
								})
							).hash
						: undefined,
					preserve_on_behalf_of: preserveOnBehalfOf,
					on_behalf_of_email: onBehalfOfEmail
				}
			})
		} else if (kind === 'app') {
			const app = await AppService.getAppByPath({
				workspace: workspaceFrom,
				path: path
			})
			if (alreadyExists) {
				if (app.raw_app) {
					const secret = await AppService.getPublicSecretOfLatestVersionOfApp({
						workspace: workspaceFrom,
						path: app.path
					})
					const js = await AppService.getRawAppData({
						secretWithExtension: `${secret}.js`,
						workspace: workspaceFrom
					})
					const css = await AppService.getRawAppData({
						secretWithExtension: `${secret}.css`,
						workspace: workspaceFrom
					})
					await AppService.updateAppRaw({
						workspace: workspaceTo,
						path: path,
						formData: {
							app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
							css,
							js
						}
					})
				} else {
					await AppService.updateApp({
						workspace: workspaceTo,
						path: path,
						requestBody: {
							...app,
							preserve_on_behalf_of: preserveOnBehalfOf
						}
					})
				}
			} else {
				if (app.raw_app) {
					const secret = await AppService.getPublicSecretOfLatestVersionOfApp({
						workspace: workspaceFrom,
						path: app.path
					})
					const js = await AppService.getRawAppData({
						secretWithExtension: `${secret}.js`,
						workspace: workspaceFrom
					})
					const css = await AppService.getRawAppData({
						secretWithExtension: `${secret}.css`,
						workspace: workspaceFrom
					})
					await AppService.createAppRaw({
						workspace: workspaceTo,
						formData: {
							app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
							css,
							js
						}
					})
				} else {
					await AppService.createApp({
						workspace: workspaceTo,
						requestBody: {
							...app,
							preserve_on_behalf_of: preserveOnBehalfOf
						}
					})
				}
			}
		} else if (kind === 'variable') {
			const variable = await VariableService.getVariable({
				workspace: workspaceFrom,
				path: path,
				decryptSecret: true
			})
			if (alreadyExists) {
				await VariableService.updateVariable({
					workspace: workspaceTo,
					path: path,
					requestBody: {
						path: path,
						value: variable.value ?? '',
						is_secret: variable.is_secret,
						description: variable.description ?? ''
					},
					alreadyEncrypted: false
				})
			} else {
				await VariableService.createVariable({
					workspace: workspaceTo,
					requestBody: {
						path: path,
						value: variable.value ?? '',
						is_secret: variable.is_secret,
						description: variable.description ?? ''
					}
				})
			}
		} else if (kind === 'resource') {
			const resource = await ResourceService.getResource({
				workspace: workspaceFrom,
				path: path
			})
			if (alreadyExists) {
				await ResourceService.updateResource({
					workspace: workspaceTo,
					path: path,
					requestBody: {
						path: path,
						value: resource.value ?? '',
						description: resource.description ?? ''
					}
				})
			} else {
				await ResourceService.createResource({
					workspace: workspaceTo,
					requestBody: {
						path: path,
						value: resource.value ?? '',
						resource_type: resource.resource_type,
						description: resource.description ?? ''
					}
				})
			}
		} else if (kind === 'resource_type') {
			const resource = await ResourceService.getResourceType({
				workspace: workspaceFrom,
				path: path
			})
			if (alreadyExists) {
				await ResourceService.updateResourceType({
					workspace: workspaceTo,
					path: path,
					requestBody: {
						schema: resource.schema,
						description: resource.description ?? ''
					}
				})
			} else {
				await ResourceService.createResourceType({
					workspace: workspaceTo,
					requestBody: {
						description: resource.description ?? '',
						schema: resource.schema,
						name: resource.name
					}
				})
			}
		} else if (kind === 'folder') {
			await FolderService.createFolder({
				workspace: workspaceTo,
				requestBody: {
					name: path
				}
			})
		} else if (kind === 'trigger') {
			if (additionalInformation?.triggers) {
				const { data, createFn, updateFn } = await getTriggersDeployData(
					additionalInformation.triggers.kind,
					path,
					workspaceFrom,
					onBehalfOfEmail
				)
				if (alreadyExists) {
					await updateFn({
						path,
						workspace: workspaceTo,
						requestBody: data
					} as any)
				} else {
					await createFn({
						workspace: workspaceTo,
						requestBody: data
					} as any)
				}
			} else {
				throw new Error('Missing triggers kind')
			}
		} else {
			throw new Error(`Unknown kind ${kind}`)
		}

		return { success: true }
	} catch (e: any) {
		return { success: false, error: e.body || e.message }
	}
}

/**
 * Check if an item already exists in the target workspace.
 */
export async function checkItemExists(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<boolean> {
	if (kind === 'flow') {
		return await FlowService.existsFlowByPath({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'script') {
		return await ScriptService.existsScriptByPath({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'app') {
		return await AppService.existsApp({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'variable') {
		return await VariableService.existsVariable({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'resource') {
		return await ResourceService.existsResource({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'schedule') {
		return await ScheduleService.existsSchedule({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'resource_type') {
		return await ResourceService.existsResourceType({
			workspace: workspace,
			path: path
		})
	} else if (kind === 'folder') {
		return await FolderService.existsFolder({
			workspace: workspace,
			name: path
		})
	} else if (kind === 'trigger') {
		const triggersKind: TriggerKind[] = [
			'kafka',
			'mqtt',
			'nats',
			'postgres',
			'routes',
			'schedules',
			'sqs',
			'websockets',
			'gcp'
		]
		if (
			additionalInformation?.triggers &&
			triggersKind.includes(additionalInformation.triggers.kind)
		) {
			return await existsTrigger(
				{ workspace: workspace, path },
				additionalInformation.triggers.kind
			)
		} else {
			throw new Error(
				`Unexpected triggers kind, expected one of: '${triggersKind.join(', ')}' got: ${
					additionalInformation?.triggers?.kind
				}`
			)
		}
	} else {
		throw new Error(`Unknown kind ${kind}`)
	}
}

/**
 * Get the value of an item for diff comparison.
 */
export async function getItemValue(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<unknown> {
	try {
		if (kind === 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace: workspace,
				path: path
			})
			getAllModules(flow.value.modules).forEach((x) => {
				if (x.value.type === 'script' && x.value.hash != undefined) {
					x.value.hash = undefined
				}
			})
			return { summary: flow.summary, description: flow.description, value: flow.value }
		} else if (kind === 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace: workspace,
				path: path
			})
			return {
				content: script.content,
				lock: script.lock,
				schema: script.schema,
				summary: script.summary,
				language: script.language
			}
		} else if (kind === 'app') {
			const app = await AppService.getAppByPath({
				workspace: workspace,
				path: path
			})
			return app
		} else if (kind === 'variable') {
			const variable = await VariableService.getVariable({
				workspace: workspace,
				path: path,
				decryptSecret: true
			})
			return variable.value
		} else if (kind === 'resource') {
			const resource = await ResourceService.getResource({
				workspace: workspace,
				path: path
			})
			return resource.value
		} else if (kind === 'resource_type') {
			const resource = await ResourceService.getResourceType({
				workspace: workspace,
				path: path
			})
			return resource.schema
		} else if (kind === 'folder') {
			const folder = await FolderService.getFolder({
				workspace: workspace,
				name: path
			})
			return {
				name: folder.name
			}
		} else if (kind === 'trigger') {
			if (additionalInformation?.triggers) {
				return await getTriggerValue(additionalInformation.triggers.kind, path, workspace)
			} else {
				throw new Error(`Missing trigger information`)
			}
		} else {
			throw new Error(`Unknown kind ${kind}`)
		}
	} catch {
		return {}
	}
}

/**
 * Get the on_behalf_of_email for a flow, script, app, or trigger (including schedule).
 */
export async function getOnBehalfOfEmail(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<string | undefined> {
	try {
		if (kind === 'flow') {
			const flow = await FlowService.getFlowByPath({ workspace, path })
			return flow.on_behalf_of_email
		} else if (kind === 'script') {
			const script = await ScriptService.getScriptByPath({ workspace, path })
			return script.on_behalf_of_email
		} else if (kind === 'app') {
			const app = await AppService.getAppByPath({ workspace, path })
			return app.policy.on_behalf_of_email
		} else if (kind === 'trigger' && additionalInformation?.triggers) {
			return await getTriggerEmail(additionalInformation.triggers.kind, path, workspace)
		}
	} catch {
		// Item may not exist in the workspace
	}
	return undefined
}
