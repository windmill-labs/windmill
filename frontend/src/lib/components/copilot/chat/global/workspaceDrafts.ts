import { AppService, DraftService, FlowService, ScriptService } from '$lib/gen'
import { DEFAULT_DATA as DEFAULT_RAW_APP_DATA } from '$lib/components/raw_apps/dataTableRefUtils'
import { appSourceToRawAppDraft } from '$lib/components/raw_apps/rawAppDraftCodec'
import { updateRawAppPolicy } from '$lib/components/raw_apps/rawAppPolicy'
import { inferArgs } from '$lib/infer'
import { emptySchema } from '$lib/utils'
import type { AppWithLastVersion, Flow, ListableApp, NewScript, Script } from '$lib/gen/types.gen'
import { UserDraft, type UserDraftMeta } from '$lib/userDraft.svelte'
import { bundleRawAppDraft } from './rawAppBundlerBridge'
import type {
	AppDraftValue,
	FlowDraftValue,
	TriggerKind,
	WorkspaceItem,
	WorkspaceItemType
} from './workspaceItems'
import {
	buildFlowDeployRequestBody,
	buildScriptDeployRequestBody,
	type FlowDeployMetadata,
	type ScriptDeployMetadata
} from './deployRequests'
import { getGlobalDraft, getGlobalDraftStoragePath, syncLiveGlobalDraft } from './userDraftAdapter'

export type DraftFlagged = {
	has_draft?: boolean
	draft_only?: boolean
}

export type ReadableScript = Pick<Script, 'path' | 'summary' | 'language' | 'content'> &
	DraftFlagged

export type DbDraftWorkspaceItemType = Extract<WorkspaceItemType, 'script' | 'flow' | 'app'>

export type DeployableDraft = {
	item: WorkspaceItem
	existingScript?: ScriptDeployMetadata
	scriptMetadata?: ScriptDeployMetadata
	existingFlow?: FlowDeployMetadata
	flowMetadata?: FlowDeployMetadata
	appExists?: boolean
}

type DeployStatusHandler = (content: string) => void

type LoadedAppDraftValue = {
	value: AppDraftValue
	meta?: UserDraftMeta
	existing?: Awaited<ReturnType<typeof AppService.getAppByPathWithDraft>>
}

export function scriptToItem(
	script: ReadableScript,
	includeValue: boolean,
	isDraft = !!script.has_draft || !!script.draft_only
): WorkspaceItem {
	return {
		type: 'script',
		path: script.path,
		summary: script.summary,
		language: script.language,
		value: includeValue ? script.content : undefined,
		isDraft
	}
}

export function flowToItem(
	flow: Pick<Flow, 'path' | 'summary' | 'value' | 'schema'> & DraftFlagged,
	includeValue: boolean,
	isDraft = !!flow.has_draft || !!flow.draft_only
): WorkspaceItem {
	return {
		type: 'flow',
		path: flow.path,
		summary: flow.summary,
		value: includeValue
			? { value: flow.value, schema: flow.schema, groups: flow.value.groups ?? null }
			: undefined,
		isDraft
	}
}

export function appToItem(
	app: (ListableApp | AppWithLastVersion) & DraftFlagged,
	includeValue: boolean
): WorkspaceItem {
	return {
		type: 'app',
		path: app.path,
		summary: app.summary,
		value: includeValue ? ((app as AppWithLastVersion).value as AppDraftValue) : undefined,
		isDraft: !!app.has_draft || !!app.draft_only
	}
}

export async function loadScriptWithDbDraft(
	path: string,
	workspace: string
): Promise<ReadableScript> {
	const script = await ScriptService.getScriptByPathWithDraft({ workspace, path })
	return script.draft ? { ...script, ...script.draft, path: script.path } : script
}

export async function loadFlowWithDbDraft(
	path: string,
	workspace: string
): Promise<Pick<Flow, 'path' | 'summary' | 'value' | 'schema'> & DraftFlagged> {
	const flow = await FlowService.getFlowByPathWithDraft({ workspace, path })
	return flow.draft ? { ...flow, ...flow.draft, path: flow.path } : flow
}

export function isDbDraftWorkspaceItemType(
	type: WorkspaceItemType
): type is DbDraftWorkspaceItemType {
	return type === 'script' || type === 'flow' || type === 'app'
}

async function createDbDraft(
	workspace: string,
	typ: DbDraftWorkspaceItemType,
	path: string,
	value: unknown
): Promise<void> {
	await DraftService.createDraft({
		workspace,
		requestBody: {
			path,
			typ,
			value
		}
	})
}

async function deleteDbDraft(
	workspace: string,
	typ: DbDraftWorkspaceItemType,
	path: string
): Promise<void> {
	await DraftService.deleteDraft({ workspace, kind: typ, path })
}

export async function deleteDbDraftAndDraftOnlyAnchor(
	workspace: string,
	typ: DbDraftWorkspaceItemType,
	path: string
): Promise<void> {
	switch (typ) {
		case 'script': {
			const existing = (await ScriptService.existsScriptByPath({ workspace, path }))
				? await ScriptService.getScriptByPathWithDraft({ workspace, path })
				: undefined
			if (existing?.draft_only) {
				await ScriptService.deleteScriptByPath({ workspace, path, keepCaptures: true })
			}
			await deleteDbDraft(workspace, typ, path)
			break
		}
		case 'flow': {
			const existing = (await FlowService.existsFlowByPath({ workspace, path }))
				? await FlowService.getFlowByPathWithDraft({ workspace, path })
				: undefined
			if (existing?.draft_only) {
				await FlowService.deleteFlowByPath({ workspace, path, keepCaptures: true })
			}
			await deleteDbDraft(workspace, typ, path)
			break
		}
		case 'app': {
			const existing = (await AppService.existsApp({ workspace, path }))
				? await AppService.getAppByPathWithDraft({ workspace, path })
				: undefined
			if (existing?.draft_only) {
				await AppService.deleteApp({ workspace, path })
			}
			await deleteDbDraft(workspace, typ, path)
			break
		}
	}
}

export async function loadDbDraftForDeploy(
	workspace: string,
	type: DbDraftWorkspaceItemType,
	path: string
): Promise<DeployableDraft | undefined> {
	switch (type) {
		case 'script': {
			if (!(await ScriptService.existsScriptByPath({ workspace, path }))) return undefined
			const script = await ScriptService.getScriptByPathWithDraft({ workspace, path })
			if (!script.draft && !script.draft_only) return undefined
			const draftScript = script.draft ? { ...script, ...script.draft, path: script.path } : script
			return {
				item: scriptToItem(draftScript, true, true),
				existingScript: script,
				scriptMetadata: draftScript
			}
		}
		case 'flow': {
			if (!(await FlowService.existsFlowByPath({ workspace, path }))) return undefined
			const flow = await FlowService.getFlowByPathWithDraft({ workspace, path })
			if (!flow.draft && !flow.draft_only) return undefined
			const draftFlow = flow.draft ? { ...flow, ...flow.draft, path: flow.path } : flow
			return {
				item: flowToItem(draftFlow, true, true),
				existingFlow: flow,
				flowMetadata: draftFlow
			}
		}
		case 'app': {
			if (!(await AppService.existsApp({ workspace, path }))) return undefined
			const app = await AppService.getAppByPathWithDraft({ workspace, path })
			if (!app.draft && !app.draft_only) return undefined
			const value = appSourceToDraftValue(app.draft ?? app, app)
			return {
				item: {
					type: 'app',
					path: app.path,
					summary: value.summary,
					value,
					isDraft: true
				},
				appExists: true
			}
		}
	}
}

export async function loadDbDraftItem(
	workspace: string,
	type: DbDraftWorkspaceItemType,
	path: string
): Promise<WorkspaceItem | undefined> {
	return (await loadDbDraftForDeploy(workspace, type, path))?.item
}

export async function loadWorkspaceDraft(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<WorkspaceItem | undefined> {
	const draft = getGlobalDraft(workspace, type, path, triggerKind)
	if (draft) return draft
	if (!isDbDraftWorkspaceItemType(type)) return undefined
	return loadDbDraftItem(workspace, type, path)
}

export async function loadWorkspaceDraftForDeploy(
	workspace: string,
	type: WorkspaceItemType,
	path: string,
	triggerKind?: TriggerKind
): Promise<DeployableDraft | undefined> {
	const item = getGlobalDraft(workspace, type, path, triggerKind)
	if (item) {
		const storagePath = getGlobalDraftStoragePath(workspace, type, path, triggerKind)
		switch (type) {
			case 'script':
				return {
					item,
					existingScript: (await ScriptService.existsScriptByPath({ workspace, path }))
						? await ScriptService.getScriptByPath({ workspace, path })
						: undefined,
					scriptMetadata: UserDraft.get<NewScript>('script', storagePath, { workspace })
				}
			case 'flow':
				return {
					item,
					existingFlow: (await FlowService.existsFlowByPath({ workspace, path }))
						? await FlowService.getFlowByPath({ workspace, path })
						: undefined,
					flowMetadata: UserDraft.get<Flow>('flow', storagePath, { workspace })
				}
			case 'app':
				return { item, appExists: await AppService.existsApp({ workspace, path }) }
			default:
				return { item }
		}
	}
	if (!isDbDraftWorkspaceItemType(type)) return undefined
	return loadDbDraftForDeploy(workspace, type, path)
}

export async function deployScriptDraft(args: {
	workspace: string
	path: string
	draft: WorkspaceItem
	existing?: ScriptDeployMetadata
	deploymentMessage?: string
	draftMetadata?: ScriptDeployMetadata
}): Promise<void> {
	const { workspace, path, draft, existing, deploymentMessage, draftMetadata } = args
	const requestBody = buildScriptDeployRequestBody(
		path,
		draft,
		existing,
		deploymentMessage,
		draftMetadata
	)
	try {
		const schema = emptySchema()
		await inferArgs(requestBody.language, requestBody.content, schema)
		requestBody.schema = schema
	} catch (e) {
		console.error('Failed to infer script schema before deploy', e)
	}
	await ScriptService.createScript({ workspace, requestBody })
}

export async function deployFlowDraft(args: {
	workspace: string
	path: string
	draft: WorkspaceItem
	existing?: FlowDeployMetadata
	deploymentMessage?: string
	draftMetadata?: FlowDeployMetadata
}): Promise<void> {
	const { workspace, path, draft, existing, deploymentMessage, draftMetadata } = args
	const flowDraft = draft.value as FlowDraftValue
	const requestBody = buildFlowDeployRequestBody(
		path,
		draft.summary,
		flowDraft,
		existing,
		deploymentMessage,
		draftMetadata
	)
	if (existing) {
		await FlowService.updateFlow({ workspace, path, requestBody })
	} else {
		await FlowService.createFlow({ workspace, requestBody })
	}
}

export async function deployAppDraft(args: {
	workspace: string
	path: string
	draft: WorkspaceItem
	appExists?: boolean
	deploymentMessage?: string
	onStatus?: DeployStatusHandler
}): Promise<void> {
	const { workspace, path, draft, appExists, deploymentMessage, onStatus } = args
	const appDraft = draft.value as AppDraftValue
	const appValue: AppDraftValue = {
		...appDraft,
		files: { ...(appDraft.files ?? {}) },
		runnables: { ...(appDraft.runnables ?? {}) },
		data: appDraft.data ?? { ...DEFAULT_RAW_APP_DATA }
	}
	await recomputeAppPolicy(appValue)
	const policy = appValue.policy
	if (!policy) {
		throw new Error(`Draft app "${path}" has no policy to deploy.`)
	}

	onStatus?.(`Bundling app "${path}"...`)
	const bundle = await bundleRawAppDraft({
		workspace,
		files: appValue.files,
		onLog: (delta) => {
			const lines = delta
				.split('\n')
				.map((line) => line.trim())
				.filter(Boolean)
			const latest = lines[lines.length - 1]
			if (latest) {
				onStatus?.(`Bundling app "${path}"... ${latest}`)
			}
		}
	})

	onStatus?.(`Deploying app "${path}"...`)
	const rawAppValue = {
		files: appValue.files,
		runnables: appValue.runnables,
		data: appValue.data ?? { ...DEFAULT_RAW_APP_DATA }
	}
	const summary = appValue.summary ?? draft.summary ?? ''
	if (appExists ?? (await AppService.existsApp({ workspace, path }))) {
		// Omit custom_path on update for now. The backend preserves it when absent, while
		// sending it requires admin privileges; this chat deploy path does not yet mirror
		// the raw app editor's user/admin-specific custom_path handling.
		await AppService.updateAppRaw({
			workspace,
			path,
			formData: {
				app: {
					path,
					value: rawAppValue,
					summary,
					policy,
					deployment_message: deploymentMessage
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	} else {
		await AppService.createAppRaw({
			workspace,
			formData: {
				app: {
					path,
					value: rawAppValue,
					summary,
					policy,
					deployment_message: deploymentMessage,
					custom_path: appValue.custom_path
				},
				js: bundle.js,
				css: bundle.css
			}
		})
	}
}

export async function saveScriptDraftToDb(
	workspace: string,
	path: string,
	draft: NewScript,
	existing: Awaited<ReturnType<typeof ScriptService.getScriptByPathWithDraft>> | undefined
): Promise<void> {
	if (!existing) {
		await ScriptService.createScript({
			workspace,
			requestBody: {
				...draft,
				path,
				parent_hash: undefined,
				draft_only: true
			}
		})
	}
	await createDbDraft(workspace, 'script', path, draft)
}

export async function saveFlowDraftToDb(
	workspace: string,
	path: string,
	draft: Flow,
	existing: Awaited<ReturnType<typeof FlowService.getFlowByPathWithDraft>> | undefined
): Promise<void> {
	if (!existing) {
		await FlowService.createFlow({
			workspace,
			requestBody: {
				path,
				summary: draft.summary ?? '',
				description: (draft as any).description ?? '',
				value: draft.value,
				schema: draft.schema,
				tag: draft.tag,
				draft_only: true,
				ws_error_handler_muted: draft.ws_error_handler_muted,
				visible_to_runner_only: draft.visible_to_runner_only,
				on_behalf_of_email: draft.on_behalf_of_email,
				labels: draft.labels
			}
		})
	}
	await createDbDraft(workspace, 'flow', path, draft)
}

function appDraftToDbValue(path: string, value: AppDraftValue): Record<string, unknown> {
	return {
		value: {
			files: value.files,
			runnables: value.runnables,
			data: value.data ?? { ...DEFAULT_RAW_APP_DATA }
		},
		summary: value.summary ?? '',
		policy: value.policy,
		path,
		custom_path: value.custom_path
	}
}

async function createRawAppDraftOnlyAnchor(
	workspace: string,
	path: string,
	value: AppDraftValue
): Promise<void> {
	await recomputeAppPolicy(value)
	const policy = value.policy
	if (!policy) {
		throw new Error(`Draft app "${path}" has no policy to save.`)
	}
	const bundle = await bundleRawAppDraft({
		workspace,
		files: value.files,
		onLog: () => {}
	})
	await AppService.createAppRaw({
		workspace,
		formData: {
			app: {
				path,
				value: {
					files: value.files,
					runnables: value.runnables,
					data: value.data ?? { ...DEFAULT_RAW_APP_DATA }
				},
				summary: value.summary ?? '',
				policy,
				draft_only: true,
				custom_path: value.custom_path
			},
			js: bundle.js,
			css: bundle.css
		}
	})
}

async function saveAppDraftToDb(
	workspace: string,
	path: string,
	value: AppDraftValue,
	existing: Awaited<ReturnType<typeof AppService.getAppByPathWithDraft>> | undefined
): Promise<void> {
	const current =
		existing ??
		((await AppService.existsApp({ workspace, path }))
			? await AppService.getAppByPathWithDraft({ workspace, path })
			: undefined)
	if (!current) {
		await createRawAppDraftOnlyAnchor(workspace, path, value)
	}
	await createDbDraft(workspace, 'app', path, appDraftToDbValue(path, value))
}

function appSourceToDraftValue(app: any, fallback?: any): AppDraftValue {
	return appSourceToRawAppDraft(app, fallback)
}

function appDraftMeta(app: { versions?: number[]; draft_created_at?: string }): UserDraftMeta {
	return {
		remoteRev: app.versions ? app.versions[app.versions.length - 1] : undefined,
		remoteDraftRev: app.draft_created_at
	}
}

export async function loadAppValueForRead(path: string, workspace: string): Promise<AppDraftValue> {
	const draft = getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return draft.value as AppDraftValue
	}

	const app = await AppService.getAppByPathWithDraft({ workspace, path })
	return appSourceToDraftValue(app.draft ?? app, app)
}

export async function loadAppDraftValue(
	path: string,
	workspace: string
): Promise<LoadedAppDraftValue> {
	const draft = getGlobalDraft(workspace, 'app', path)
	if (draft && draft.value && typeof draft.value === 'object' && 'files' in draft.value) {
		return { value: draft.value as AppDraftValue }
	}

	const app = await AppService.getAppByPathWithDraft({ workspace, path })
	const value = appSourceToDraftValue(app.draft ?? app, app)
	return { value, meta: appDraftMeta(app), existing: app }
}

export async function saveAppDraft(
	workspace: string,
	path: string,
	value: AppDraftValue,
	meta?: UserDraftMeta,
	existing?: Awaited<ReturnType<typeof AppService.getAppByPathWithDraft>>
): Promise<WorkspaceItem> {
	await saveAppDraftToDb(workspace, path, value, existing)
	const mirrored = syncLiveGlobalDraft(workspace, 'app', path, value, meta)
	if (mirrored) return mirrored
	return {
		type: 'app',
		path,
		summary: value.summary,
		value,
		isDraft: true
	}
}

export async function recomputeAppPolicy(value: AppDraftValue): Promise<void> {
	const policy = (await updateRawAppPolicy(
		value.runnables as any,
		value.policy as any
	)) as NonNullable<AppDraftValue['policy']>
	if (!policy.execution_mode) {
		policy.execution_mode = 'publisher'
	}
	value.policy = policy
}
