import {
	AppService,
	AzureTriggerService,
	EmailTriggerService,
	FlowService,
	FolderService,
	GcpTriggerService,
	HttpTriggerService,
	KafkaTriggerService,
	MqttTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ResourceService,
	ScheduleService,
	ScriptService,
	SqsTriggerService,
	UserService,
	VariableService,
	WebsocketTriggerService,
	WorkspaceService,
	type User
} from '$lib/gen'
import {
	fetchProtectionRulesForWorkspace,
	canUserBypassRuleKindInRulesets
} from '$lib/workspaceProtectionRules.svelte'
import {
	existsTrigger,
	getTriggersDeployData,
	getTriggerPermissionedAs,
	getTriggerValue,
	stripOperationalState,
	type AdditionalInformation,
	type Kind
} from '$lib/utils_deployable'

import {
	deployItem as sharedDeployItem,
	deleteItemInWorkspace as sharedDeleteItem,
	checkItemExists as sharedCheckItemExists,
	getOnBehalfOf as sharedGetOnBehalfOf,
	getItemValue as sharedGetItemValue,
	type DeployProvider,
	type DeployKind,
	type DeployResult,
	type TriggerDeployKind
} from 'windmill-utils-internal'

export type { DeployResult, DeployKind, TriggerDeployKind }

// ---------------------------------------------------------------------------
// Provider adapter — wraps frontend's class-based services
// ---------------------------------------------------------------------------

/**
 * Map a shared `TriggerDeployKind` (e.g. `kafka_trigger`) to the per-kind
 * service class. Per-kind dispatch lives in the adapter so the shared
 * `deployItem` only needs to know "trigger" vs "not trigger".
 */
function triggerServiceFor(kind: TriggerDeployKind) {
	switch (kind) {
		case 'http_trigger':
			return {
				exists: HttpTriggerService.existsHttpTrigger,
				get: HttpTriggerService.getHttpTrigger,
				create: HttpTriggerService.createHttpTrigger,
				update: HttpTriggerService.updateHttpTrigger,
				delete: HttpTriggerService.deleteHttpTrigger
			}
		case 'websocket_trigger':
			return {
				exists: WebsocketTriggerService.existsWebsocketTrigger,
				get: WebsocketTriggerService.getWebsocketTrigger,
				create: WebsocketTriggerService.createWebsocketTrigger,
				update: WebsocketTriggerService.updateWebsocketTrigger,
				delete: WebsocketTriggerService.deleteWebsocketTrigger
			}
		case 'kafka_trigger':
			return {
				exists: KafkaTriggerService.existsKafkaTrigger,
				get: KafkaTriggerService.getKafkaTrigger,
				create: KafkaTriggerService.createKafkaTrigger,
				update: KafkaTriggerService.updateKafkaTrigger,
				delete: KafkaTriggerService.deleteKafkaTrigger
			}
		case 'nats_trigger':
			return {
				exists: NatsTriggerService.existsNatsTrigger,
				get: NatsTriggerService.getNatsTrigger,
				create: NatsTriggerService.createNatsTrigger,
				update: NatsTriggerService.updateNatsTrigger,
				delete: NatsTriggerService.deleteNatsTrigger
			}
		case 'postgres_trigger':
			return {
				exists: PostgresTriggerService.existsPostgresTrigger,
				get: PostgresTriggerService.getPostgresTrigger,
				create: PostgresTriggerService.createPostgresTrigger,
				update: PostgresTriggerService.updatePostgresTrigger,
				delete: PostgresTriggerService.deletePostgresTrigger
			}
		case 'mqtt_trigger':
			return {
				exists: MqttTriggerService.existsMqttTrigger,
				get: MqttTriggerService.getMqttTrigger,
				create: MqttTriggerService.createMqttTrigger,
				update: MqttTriggerService.updateMqttTrigger,
				delete: MqttTriggerService.deleteMqttTrigger
			}
		case 'sqs_trigger':
			return {
				exists: SqsTriggerService.existsSqsTrigger,
				get: SqsTriggerService.getSqsTrigger,
				create: SqsTriggerService.createSqsTrigger,
				update: SqsTriggerService.updateSqsTrigger,
				delete: SqsTriggerService.deleteSqsTrigger
			}
		case 'gcp_trigger':
			return {
				exists: GcpTriggerService.existsGcpTrigger,
				get: GcpTriggerService.getGcpTrigger,
				create: GcpTriggerService.createGcpTrigger,
				update: GcpTriggerService.updateGcpTrigger,
				delete: GcpTriggerService.deleteGcpTrigger
			}
		case 'azure_trigger':
			return {
				exists: AzureTriggerService.existsAzureTrigger,
				get: AzureTriggerService.getAzureTrigger,
				create: AzureTriggerService.createAzureTrigger,
				update: AzureTriggerService.updateAzureTrigger,
				delete: AzureTriggerService.deleteAzureTrigger
			}
		case 'email_trigger':
			return {
				exists: EmailTriggerService.existsEmailTrigger,
				get: EmailTriggerService.getEmailTrigger,
				create: EmailTriggerService.createEmailTrigger,
				update: EmailTriggerService.updateEmailTrigger,
				delete: EmailTriggerService.deleteEmailTrigger
			}
		default: {
			// Exhaustiveness guard: extending TriggerDeployKind without a case here
			// produces a compile error rather than a silent runtime failure.
			const _exhaustive: never = kind
			throw new Error(`Unhandled trigger kind: ${_exhaustive}`)
		}
	}
}

/**
 * Map the shared `TriggerDeployKind` to the legacy frontend `TriggerKind`
 * used by helpers in `utils_deployable.ts`.
 */
function legacyTriggerKind(kind: TriggerDeployKind) {
	const map = {
		http_trigger: 'routes',
		websocket_trigger: 'websockets',
		kafka_trigger: 'kafka',
		nats_trigger: 'nats',
		postgres_trigger: 'postgres',
		mqtt_trigger: 'mqtt',
		sqs_trigger: 'sqs',
		gcp_trigger: 'gcp',
		azure_trigger: 'azure',
		email_trigger: 'emails'
	} as const
	return map[kind]
}

/** An identity in both formats the app policy stores it in. */
export type AppIdentity = { email: string; permissionedAs: string }

/**
 * `deployItem` overrides only the email half of the identity, while the body it builds
 * spreads the *source* item — which carries the source workspace's permissioned_as, valid
 * nowhere else since usernames are per-workspace. The key is therefore always overwritten:
 * with the picked user's principal for a custom choice, and cleared otherwise so the
 * backend derives the target's own from the email it is given. The shared `deployItem`
 * clears it too, but this app consumes the published package, so the clear has to exist
 * on both sides until that version ships.
 */
function makeProvider(onBehalfOfPrincipal?: string, appIdentity?: AppIdentity): DeployProvider {
	const withPermissionedAs = <T extends Record<string, any>>(requestBody: T): T => ({
		...requestBody,
		on_behalf_of: onBehalfOfPrincipal
	})
	return {
		existsFlowByPath: (p) => FlowService.existsFlowByPath(p),
		existsScriptByPath: (p) => ScriptService.existsScriptByPath(p),
		existsApp: (p) => AppService.existsApp(p),
		existsVariable: (p) => VariableService.existsVariable(p),
		existsResource: (p) => ResourceService.existsResource(p),
		existsResourceType: (p) => ResourceService.existsResourceType(p),
		existsFolder: (p) => FolderService.existsFolder(p),
		getFlowByPath: (p) => FlowService.getFlowByPath(p),
		createFlow: (p) =>
			FlowService.createFlow({ ...p, requestBody: withPermissionedAs(p.requestBody) }),
		updateFlow: (p) =>
			FlowService.updateFlow({ ...p, requestBody: withPermissionedAs(p.requestBody) }),
		archiveFlowByPath: (p) => FlowService.archiveFlowByPath(p),
		getScriptByPath: (p) => ScriptService.getScriptByPath(p),
		createScript: (p) =>
			ScriptService.createScript({ ...p, requestBody: withPermissionedAs(p.requestBody) }),
		archiveScriptByPath: (p) => ScriptService.archiveScriptByPath(p),
		// An app's identity lives in its policy, and the shared deploy forwards the source policy
		// untouched — it only turns `onBehalfOf` into `preserve_on_behalf_of: true`. Rewriting the
		// policy on the way out is therefore the only way a chosen identity reaches the target; the
		// backend honours it (`should_preserve` requires `policy.on_behalf_of.is_some()`).
		getAppByPath: async (p) => {
			const app = await AppService.getAppByPath(p)
			if (!appIdentity) return app
			return {
				...app,
				policy: {
					...app.policy,
					on_behalf_of: appIdentity.permissionedAs,
					on_behalf_of_email: appIdentity.email
				}
			}
		},
		createApp: (p) => AppService.createApp(p),
		updateApp: (p) => AppService.updateApp(p),
		createAppRaw: (p) => AppService.createAppRaw(p),
		updateAppRaw: (p) => AppService.updateAppRaw(p),
		getPublicSecretOfLatestVersionOfApp: (p) => AppService.getPublicSecretOfLatestVersionOfApp(p),
		getRawAppData: (p) => AppService.getRawAppData(p),
		deleteApp: (p) => AppService.deleteApp(p),
		getVariable: (p) => VariableService.getVariable(p),
		createVariable: (p) => VariableService.createVariable(p),
		updateVariable: (p) => VariableService.updateVariable(p),
		deleteVariable: (p) => VariableService.deleteVariable(p),
		getResource: (p) => ResourceService.getResource(p),
		createResource: (p) => ResourceService.createResource(p),
		updateResource: (p) => ResourceService.updateResource(p),
		deleteResource: (p) => ResourceService.deleteResource(p),
		getResourceType: (p) => ResourceService.getResourceType(p),
		createResourceType: (p) => ResourceService.createResourceType(p),
		updateResourceType: (p) => ResourceService.updateResourceType(p),
		deleteResourceType: (p) => ResourceService.deleteResourceType(p),
		getFolder: (p) => FolderService.getFolder(p),
		createFolder: (p) => FolderService.createFolder(p),
		updateFolder: (p) => FolderService.updateFolder(p),
		deleteFolder: (p) => FolderService.deleteFolder(p),
		// Triggers
		existsTriggerByKind: (kind, p) => triggerServiceFor(kind).exists(p),
		getTriggerForDeploy: async (kind, p) => {
			// Reuses the existing per-kind transform map (e.g. GCP wipes
			// subscription_id and computes base_endpoint from window.location).
			// Operational-state strip is applied by the shared `deployItem`
			// after this returns.
			const { data } = await getTriggersDeployData(
				legacyTriggerKind(kind),
				p.path,
				p.workspace,
				p.onBehalfOf
			)
			return data
		},
		createTriggerByKind: (kind, p) => triggerServiceFor(kind).create(p as any),
		updateTriggerByKind: (kind, p) => triggerServiceFor(kind).update(p as any),
		deleteTriggerByKind: (kind, p) => triggerServiceFor(kind).delete(p),
		getTriggerValue: (kind, p) => getTriggerValue(legacyTriggerKind(kind), p.path, p.workspace),
		getTriggerPermissionedAs: async (kind, p) => {
			const trigger = await triggerServiceFor(kind).get(p)
			return (trigger as any)?.permissioned_as
		},
		// Schedules
		existsSchedule: (p) => ScheduleService.existsSchedule(p),
		getSchedule: (p) => ScheduleService.getSchedule(p),
		createSchedule: (p) => ScheduleService.createSchedule(p),
		updateSchedule: (p) => ScheduleService.updateSchedule(p),
		deleteSchedule: (p) => ScheduleService.deleteSchedule(p),
		// Datatable migrations
		listDatatableMigrations: (p) => WorkspaceService.listDatatableMigrations(p),
		upsertDatatableMigration: (p) => WorkspaceService.upsertDatatableMigration(p),
		deleteDatatableMigration: (p) => WorkspaceService.deleteDatatableMigration(p)
	}
}

// ---------------------------------------------------------------------------
// Public API — thin wrappers over the shared dispatch
// ---------------------------------------------------------------------------

export interface DeployItemParams {
	kind: Kind
	path: string
	workspaceFrom: string
	workspaceTo: string
	/**
	 * Carries the trigger sub-kind for the legacy generic `kind: 'trigger'` path
	 * used by `DeployWorkspace.svelte`. Not needed for the new per-kind names
	 * (`http_trigger`, `kafka_trigger`, …) returned by the fork-merge compare API.
	 */
	additionalInformation?: AdditionalInformation
	/**
	 * The value to use for on_behalf_of when deploying.
	 * Format varies by item kind:
	 * - For flows/scripts/apps: an email address (on_behalf_of_email)
	 * - For triggers/schedules: permissioned_as format (u/username or g/group)
	 * If set, preserve_on_behalf_of / preserve_permissioned_as will be true.
	 * If undefined, the deploying user's identity is used.
	 */
	onBehalfOf?: string
	/**
	 * Authorization half of `onBehalfOf` (u/username or g/group). Must name the same identity as
	 * `onBehalfOf`. Set it only when the user picked a specific user; undefined clears the key,
	 * leaving the backend to derive the target workspace's own principal from `onBehalfOf`. Apps
	 * additionally need it in the policy, which holds both formats — see `makeProvider`.
	 */
	onBehalfOfPrincipal?: string
}

/**
 * Deploy an item from one workspace to another. Handles every kind in the shared
 * `DeployKind` union plus the legacy generic `'trigger'` from `DeployWorkspace.svelte`,
 * which carries its sub-kind in `additionalInformation`.
 */
export async function deployItem(params: DeployItemParams): Promise<DeployResult> {
	const {
		kind,
		path,
		workspaceFrom,
		workspaceTo,
		additionalInformation,
		onBehalfOf,
		onBehalfOfPrincipal
	} = params

	if (kind === 'trigger') {
		// Legacy path: `DeployWorkspace.svelte` doesn't know the per-kind trigger
		// name when building dependency graphs, so it passes `kind: 'trigger'` and
		// the actual sub-kind in `additionalInformation`. Translate to per-kind.
		if (!additionalInformation?.triggers) {
			return { success: false, error: 'Missing triggers kind' }
		}
		try {
			const alreadyExists = await existsTrigger(
				{ workspace: workspaceTo, path },
				additionalInformation.triggers.kind
			)
			const { data, createFn, updateFn } = await getTriggersDeployData(
				additionalInformation.triggers.kind,
				path,
				workspaceFrom,
				onBehalfOf
			)
			if (alreadyExists) {
				// Strip operational state so the update doesn't flip the target's
				// existing enabled/mode flag — preserved via `is_mode_unspecified()`
				// on the backend. Mirrors the shared `stripOperationalStateOnUpdate`
				// in the merge-deploy path.
				const stripped = stripOperationalState(data)
				await updateFn({ path, workspace: workspaceTo, requestBody: stripped } as any)
			} else {
				// Create — pass source's `mode`/`enabled` through so a new
				// trigger lands with the state the source workspace had.
				await createFn({ workspace: workspaceTo, requestBody: data } as any)
			}
			return { success: true }
		} catch (e: any) {
			return { success: false, error: e.body || e.message || String(e) }
		}
	}

	const appIdentity =
		(kind === 'app' || kind === 'raw_app') && onBehalfOf && onBehalfOfPrincipal
			? { email: onBehalfOf, permissionedAs: onBehalfOfPrincipal }
			: undefined
	return sharedDeployItem(
		makeProvider(onBehalfOfPrincipal, appIdentity),
		kind as DeployKind,
		path,
		workspaceFrom,
		workspaceTo,
		onBehalfOf
	)
}

/**
 * The two sides of a `workspace_diff` row a deploy direction reads:
 * `exists_in_source` is the parent (or arbitrary target) side, `exists_in_fork`
 * the current workspace. `fork_last_event_*` is what the tally recorded for the
 * fork's last write at this path, absent on a row that predates the recording.
 */
type WorkspaceDiffSides = {
	ahead: number
	behind: number
	exists_in_source: boolean
	exists_in_fork: boolean
	fork_last_event_kind?: 'write' | 'delete' | 'rename_from'
	fork_last_event_origin?: 'authored' | 'sync'
}

/** Deploying this row creates the item in the target, which does not have it. */
export function diffCreatesInTarget(diff: WorkspaceDiffSides, mergeIntoParent: boolean): boolean {
	return mergeIntoParent ? diff.exists_in_source === false : diff.exists_in_fork === false
}

/**
 * Deploying this row removes the item in the target, the only side that has it.
 * A removal is always opt-in (never bulk-selected): the row states what deploying
 * does, and for the merge direction `diffForkDroppedItem` is what justifies
 * offering it at all.
 */
export function diffRemovesInTarget(diff: WorkspaceDiffSides, mergeIntoParent: boolean): boolean {
	return mergeIntoParent ? diff.exists_in_fork === false : diff.exists_in_source === false
}

/**
 * The fork dropped the item on purpose — someone deleted it, or renamed it away.
 * The counters cannot show this (they count writes on a side without saying what
 * they were), so only the recorded event does: a sync-origin removal is a git-sync
 * revert rather than a fork decision, and an unrecorded one is no evidence at all.
 */
export function diffForkDroppedItem(diff: WorkspaceDiffSides): boolean {
	return (
		diff.fork_last_event_origin === 'authored' &&
		(diff.fork_last_event_kind === 'delete' || diff.fork_last_event_kind === 'rename_from')
	)
}

/**
 * Rows a deploy in this direction can act on. A merge carries what the fork *has*,
 * plus what it can show it dropped; an item the fork merely never received is no
 * fork change. The update direction takes a parent-only row whatever the counters
 * say. An arbitrary target merges one unconditionally — that one-way sync has no
 * tally, so target-only does mean "remove".
 */
export function diffActionableInDirection(
	diff: WorkspaceDiffSides,
	mergeIntoParent: boolean,
	isArbitraryTarget: boolean = false
): boolean {
	if (mergeIntoParent) {
		if (!isArbitraryTarget && diff.exists_in_fork === false && !diffForkDroppedItem(diff)) {
			return false
		}
		return diff.ahead > 0
	}
	return diff.behind > 0 || diffCreatesInTarget(diff, mergeIntoParent)
}

/**
 * Delete/archive an item in a workspace.
 * Used when deploying a deletion from one workspace to another.
 * Scripts and flows are archived (reversible). Other types are deleted.
 */
export async function deleteItemInWorkspace(
	kind: Kind,
	path: string,
	workspace: string
): Promise<DeployResult> {
	return sharedDeleteItem(makeProvider(), kind as DeployKind, path, workspace)
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
	if (kind === 'trigger') {
		if (!additionalInformation?.triggers) {
			throw new Error('Missing triggers kind for legacy trigger deploy')
		}
		return existsTrigger({ workspace, path }, additionalInformation.triggers.kind)
	}
	return sharedCheckItemExists(makeProvider(), kind as DeployKind, path, workspace)
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
	if (kind === 'trigger') {
		if (!additionalInformation?.triggers) return {}
		try {
			return await getTriggerValue(additionalInformation.triggers.kind, path, workspace)
		} catch {
			return {}
		}
	}
	return sharedGetItemValue(makeProvider(), kind as DeployKind, path, workspace)
}

/**
 * Get the on_behalf_of value for a deployable item.
 */
export async function getOnBehalfOf(
	kind: Kind,
	path: string,
	workspace: string,
	additionalInformation?: AdditionalInformation
): Promise<string | undefined> {
	if (kind === 'trigger' && additionalInformation?.triggers) {
		try {
			return await getTriggerPermissionedAs(additionalInformation.triggers.kind, path, workspace)
		} catch {
			return undefined
		}
	}
	return sharedGetOnBehalfOf(makeProvider(), kind as DeployKind, path, workspace)
}

/**
 * `getOnBehalfOf` without its swallow-and-return-undefined. A caller that offers an identity choice
 * only when the source has one must be able to tell "carries no identity" from "could not read it":
 * conflating them silently reassigns the deployed item to whoever deployed it.
 */
export async function getOnBehalfOfOrThrow(
	kind: 'script' | 'flow' | 'app' | 'raw_app',
	path: string,
	workspace: string
): Promise<string | undefined> {
	const provider = makeProvider()
	if (kind === 'flow') return (await provider.getFlowByPath({ workspace, path })).on_behalf_of_email
	if (kind === 'script')
		return (await provider.getScriptByPath({ workspace, path })).on_behalf_of_email
	return (await provider.getAppByPath({ workspace, path })).policy?.on_behalf_of_email
}

export type DeployPermission = { ok: boolean; reason?: string }

/**
 * Whether the current user may deploy into `workspace`. Mirrors the server-side
 * deploy authorization (`check_user_against_rule` in windmill-common) so the UI
 * can disable the action with a reason instead of letting the click 403:
 *  - operators can never deploy;
 *  - when the `RestrictDeployToDeployers` protection rule is active, only
 *    admins, `wm_deployers` members (implicitly), and per-ruleset bypass
 *    users/groups may deploy.
 * Fails open on any error — the server still enforces on the actual deploy.
 * Shared by the session dock and the compare page so both gate identically.
 */
export async function checkDeployPermission(
	workspace: string,
	/** Pre-fetched `whoami` for `workspace`, to save a round trip when the caller already has one. */
	whoami?: User
): Promise<DeployPermission> {
	try {
		const me = whoami ?? (await UserService.whoami({ workspace }))
		if (me.operator) {
			return { ok: false, reason: "You're an operator in this workspace — operators can't deploy" }
		}
		// Admins and wm_deployers members always satisfy RestrictDeployToDeployers
		// (the backend allows wm_deployers implicitly, so check it before the
		// per-ruleset bypass_users/bypass_groups fallback).
		const isDeployer = me.is_admin || (me.groups ?? []).includes('wm_deployers')
		if (!isDeployer) {
			const rulesets = await fetchProtectionRulesForWorkspace(workspace)
			const userInfo = { is_admin: !!me.is_admin, username: me.username, groups: me.groups ?? [] }
			if (!canUserBypassRuleKindInRulesets(rulesets, 'RestrictDeployToDeployers', userInfo)) {
				return {
					ok: false,
					reason: 'Only workspace admins and members of wm_deployers can deploy here'
				}
			}
		}
		return { ok: true }
	} catch {
		return { ok: true }
	}
}

/**
 * Whether `me` may write at `path` in `workspace` — the per-item half of the deploy gate, which
 * `checkDeployPermission`'s workspace-level rules don't cover. Same fail-open contract.
 */
async function checkPathWritePermission(
	workspace: string,
	path: string,
	me: User
): Promise<DeployPermission> {
	if (me.is_admin) return { ok: true }
	const owner = path.match(/^u\/([^/]+)\//)?.[1]
	if (owner) {
		return owner === me.username
			? { ok: true }
			: {
					ok: false,
					reason: `${path} is owned by u/${owner} — only they or a workspace admin can write there`
				}
	}
	const folder = path.match(/^f\/([^/]+)\//)?.[1]
	if (!folder || me.folders?.includes(folder)) return { ok: true }
	try {
		// A folder the target doesn't have yet is created by the deploy, with the deployer as its
		// owner — lacking write access to something that doesn't exist isn't a refusal.
		if (!(await checkItemExists('folder', `f/${folder}`, workspace))) return { ok: true }
	} catch {
		// Inconclusive: let the deploy decide rather than refusing on a failed probe.
		return { ok: true }
	}
	return { ok: false, reason: `You don't have write access to folder ${folder}` }
}

export type DeployTargetAccess = {
	permission: DeployPermission
	/** Whether the user may hand the item an identity other than their own. */
	canPreserveOnBehalfOf: boolean
	/** The caller as `workspace` knows them — usernames are per-workspace, emails are not. */
	me?: AppIdentity
}

/**
 * What the target workspace says about landing one item in it: the workspace-level gate, write
 * access to the item's path, and whether another identity may be preserved. Bundled so one `whoami`
 * answers all of it, and so a refusal is known before the deploy rather than as a 403 on confirm.
 */
export async function checkItemDeployAccess(
	workspace: string,
	path: string
): Promise<DeployTargetAccess> {
	let me: User
	try {
		me = await UserService.whoami({ workspace })
	} catch {
		return { permission: { ok: true }, canPreserveOnBehalfOf: false }
	}
	const workspaceLevel = await checkDeployPermission(workspace, me)
	return {
		permission: workspaceLevel.ok
			? await checkPathWritePermission(workspace, path, me)
			: workspaceLevel,
		canPreserveOnBehalfOf: me.is_admin || (me.groups ?? []).includes('wm_deployers'),
		me: { email: me.email, permissionedAs: `u/${me.username}` }
	}
}
