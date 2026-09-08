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
	GroupService,
	UserService,
	VariableService,
	WebsocketTriggerService,
	WorkspaceService,
	type User
} from '$lib/gen'
import type { UserDraftItemKind } from '$lib/gen'
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
 * Set when a create-only deploy was refused because the target turned out to already have the
 * item. Carried on an object rather than matched out of the error text: `deployItem` swallows
 * every throw into `{ success: false, error }`, so the flag is the only reliable signal.
 */
export type DeployConflict = { hit: boolean }

/**
 * `deployItem` overrides only the email half of the identity, while the body it builds
 * spreads the *source* item — which carries the source workspace's permissioned_as, valid
 * nowhere else since usernames are per-workspace. The key is therefore always overwritten:
 * with the picked user's principal for a custom choice, and cleared otherwise so the
 * backend derives the target's own from the email it is given. The shared `deployItem`
 * clears it too, but this app consumes the published package, so the clear has to exist
 * on both sides until that version ships.
 */
function makeProvider(
	onBehalfOfPrincipal?: string,
	appIdentity?: AppIdentity,
	/**
	 * Refuse the writes the shared `deployItem` reaches for only when the item already exists in
	 * the target, turning its silent switch to an update into a failure the caller can act on.
	 * The three below are exactly its `alreadyExists` branches: a flow and an app are replaced
	 * outright, and a script is given the target's head as `parent_hash`, which is what makes an
	 * otherwise identical `createScript` an update.
	 */
	conflict?: DeployConflict
): DeployProvider {
	const withPermissionedAs = <T extends Record<string, any>>(requestBody: T): T => ({
		...requestBody,
		on_behalf_of: onBehalfOfPrincipal
	})
	const refuseUpdate = (): never => {
		if (conflict) conflict.hit = true
		throw new Error('item already exists in the target workspace')
	}
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
			conflict
				? refuseUpdate()
				: FlowService.updateFlow({ ...p, requestBody: withPermissionedAs(p.requestBody) }),
		archiveFlowByPath: (p) => FlowService.archiveFlowByPath(p),
		getScriptByPath: (p) => ScriptService.getScriptByPath(p),
		createScript: (p) =>
			conflict && p.requestBody.parent_hash
				? refuseUpdate()
				: ScriptService.createScript({ ...p, requestBody: withPermissionedAs(p.requestBody) }),
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
		updateApp: (p) => (conflict ? refuseUpdate() : AppService.updateApp(p)),
		createAppRaw: (p) => AppService.createAppRaw(p),
		updateAppRaw: (p) => (conflict ? refuseUpdate() : AppService.updateAppRaw(p)),
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
	/**
	 * Fail instead of overwriting when the target turns out to already have the item. The shared
	 * deploy re-probes and silently switches to an update, so a caller that only means to create —
	 * one acting on the item being absent — has to say so or it will overwrite whoever got there
	 * between the two probes. The result then carries `conflict`.
	 */
	createOnly?: boolean
}

/**
 * Deploy an item from one workspace to another. Handles every kind in the shared
 * `DeployKind` union plus the legacy generic `'trigger'` from `DeployWorkspace.svelte`,
 * which carries its sub-kind in `additionalInformation`.
 */
export async function deployItem(
	params: DeployItemParams
): Promise<DeployResult & { conflict?: boolean }> {
	const {
		kind,
		path,
		workspaceFrom,
		workspaceTo,
		additionalInformation,
		onBehalfOf,
		onBehalfOfPrincipal,
		createOnly
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
	const conflict: DeployConflict | undefined = createOnly ? { hit: false } : undefined
	const result = await sharedDeployItem(
		makeProvider(onBehalfOfPrincipal, appIdentity, conflict),
		kind as DeployKind,
		path,
		workspaceFrom,
		workspaceTo,
		onBehalfOf
	)
	return conflict?.hit ? { ...result, conflict: true } : result
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

/**
 * Every workspace group, not just the first page.
 *
 * `listGroupNames` would be the obvious call but unions in instance groups, which folder rules do
 * not resolve against — a same-named instance group would let an unusable rule through. `listGroups`
 * reads the workspace's own `group_` rows, which is what the server checks, but it paginates: a
 * group missed here reads as "no account in the target", which now refuses a folder copy outright.
 */
async function workspaceGroupNames(workspace: string): Promise<Set<string>> {
	const PER_PAGE = 1000
	const names = new Set<string>()
	// Stops on a short page; the size check is the backstop for a server that ignores `page`.
	for (let page = 1; page <= 50; page++) {
		const batch = await GroupService.listGroups({ workspace, page, perPage: PER_PAGE })
		const before = names.size
		batch.forEach((g) => names.add(g.name))
		if (batch.length < PER_PAGE || names.size === before) break
	}
	return names
}

/**
 * Resolve a source-workspace principal into the same person or group as the target names them.
 *
 * A `u/<username>` is workspace-local: the same username in the target can be a different account,
 * so copying one verbatim can hand a folder — or an item's execution identity — to a namesake. Email
 * is the only identifier stable across workspaces, so users go source username -> email -> target
 * username, and anyone without an account there resolves to undefined for the caller to deal with.
 */
async function principalTranslator(workspaceFrom: string, workspaceTo: string) {
	const [fromUsers, toUsers, targetGroups] = await Promise.all([
		// `list_users` is unpaginated, unlike the group listing below.
		UserService.listUsers({ workspace: workspaceFrom }),
		UserService.listUsers({ workspace: workspaceTo }),
		workspaceGroupNames(workspaceTo)
	])
	const emailOfSourceUsername = new Map(fromUsers.map((u) => [u.username, u.email]))
	const targetUsernameOfEmail = new Map(toUsers.map((u) => [u.email, u.username]))

	/** The same principal as `workspaceTo` names it, or undefined when it has no account there. */
	return (principal: string): string | undefined => {
		if (principal.startsWith('u/')) {
			const email = emailOfSourceUsername.get(principal.slice(2))
			const username = email ? targetUsernameOfEmail.get(email) : undefined
			return username ? `u/${username}` : undefined
		}
		if (principal.startsWith('g/')) {
			return targetGroups.has(principal.slice(2)) ? principal : undefined
		}
		// An email is already workspace-independent; it only has to name someone there.
		return targetUsernameOfEmail.has(principal) ? principal : undefined
	}
}

export type CreateFolderResult = DeployResult & {
	/** Access dropped because its principal has no account in the target, if any. */
	droppedAccess?: string[]
}

/**
 * Copy a folder into `workspaceTo`, creating it and never updating it.
 *
 * `deployItem` re-probes and switches to `updateFolder` when the folder turns out to exist, which
 * would replace its owners and ACL with the source's. For a folder the user asked to deploy that is
 * the point; for one created on their behalf to give an item somewhere to land it would silently
 * rewrite the permissions of a folder someone else just created. Losing that race is success here —
 * the folder exists, which is all the caller needed.
 *
 * Every principal is translated into the target's own naming (see `principalTranslator`), and the
 * two kinds of unresolvable principal are treated differently because they fail differently:
 *
 *  - an **owner or ACL entry** with no account in the target is dropped. The folder ends up more
 *    restrictive than its source, never less, and `create_folder` makes the caller an owner, so
 *    nobody is locked out of what they just created.
 *  - an **identity rule** with no account in the target refuses the whole copy. Dropping it would
 *    leave the folder applying no rule where the source applied one, so an item landing inside runs
 *    as whoever deployed it — the silent substitution this prompt exists to prevent — and carrying
 *    it verbatim is worse still: the server validates a rule's shape at folder-create time but its
 *    principal's existence at item-create time, so the folder would be created and then reject
 *    every deploy into it, including the retry.
 *
 * `default_permissioned_as` and `labels` are carried at all, which the shared folder deploy drops.
 */
export async function createFolderIfAbsent(
	name: string,
	workspaceFrom: string,
	workspaceTo: string
): Promise<CreateFolderResult> {
	try {
		const folder = await FolderService.getFolder({ workspace: workspaceFrom, name })
		const rules = folder.default_permissioned_as ?? []
		const owners = folder.owners ?? []
		const acl = Object.entries((folder.extra_perms ?? {}) as Record<string, boolean>)
		const translate = await principalTranslator(workspaceFrom, workspaceTo)

		const unresolvableRule = rules.map((r) => r.permissioned_as).find((p) => !translate(p))
		if (unresolvableRule) {
			return {
				success: false,
				error:
					`f/${name} runs items on behalf of ${unresolvableRule}, which has no account in ` +
					`the target workspace. Bring the folder across from the compare page first.`
			}
		}

		const droppedAccess = [...owners, ...acl.map(([p]) => p)].filter((p) => !translate(p))
		await FolderService.createFolder({
			workspace: workspaceTo,
			requestBody: {
				name,
				owners: owners.map(translate).filter((p): p is string => !!p),
				extra_perms: Object.fromEntries(
					acl.flatMap(([p, write]) => {
						const t = translate(p)
						return t ? [[t, write] as const] : []
					})
				),
				summary: folder.summary ?? undefined,
				default_permissioned_as: rules.map((r) => ({
					...r,
					permissioned_as: translate(r.permissioned_as)!
				})),
				labels: folder.labels
			}
		})
		return { success: true, droppedAccess: droppedAccess.length ? droppedAccess : undefined }
	} catch (e) {
		// The name conflict a concurrent create produces is not part of the API contract, so ask
		// again rather than matching its message.
		try {
			if (await checkItemExists('folder', `f/${name}`, workspaceTo)) return { success: true }
		} catch {}
		return { success: false, error: `${e}` }
	}
}

/** Which term refused, so a caller can scope a refusal the server applies to only some kinds. */
export type DeployRefusal = 'operator' | 'DisableDirectDeployment' | 'RestrictDeployToDeployers'

export type DeployPermission = { ok: boolean; reason?: string; refusedBy?: DeployRefusal }

// The server reaches `check_deploy_rules` from the item handlers, and only these kinds call it
// (windmill-api-{scripts,flows,groups}, windmill-api/src/apps.rs, windmill-store/src/{resources,
// variables}.rs). Schedules and triggers hit no gate at all, so a protection rule must not
// disable them here. Exhaustive by construction: a new `Kind` fails to compile without a verdict.
const KIND_GATED_BY_DEPLOY_RULES: Record<Kind, boolean> = {
	script: true,
	flow: true,
	app: true,
	raw_app: true,
	resource: true,
	resource_type: true,
	variable: true,
	folder: true,
	schedule: false,
	http_trigger: false,
	websocket_trigger: false,
	kafka_trigger: false,
	nats_trigger: false,
	postgres_trigger: false,
	mqtt_trigger: false,
	amqp_trigger: false,
	sqs_trigger: false,
	gcp_trigger: false,
	azure_trigger: false,
	email_trigger: false,
	datatable_migration: false,
	trigger: false,
	data_pipeline: false
}

// Every gated kind is spelled identically in `Kind` and `UserDraftItemKind`, so one lookup serves
// both taxonomies and the drafts surface needs no bridge. The kinds whose spellings diverge
// (`trigger_http` vs `http_trigger`) are exactly the ungated ones — so if a trigger kind ever
// becomes gated server-side, it needs its draft spelling added here too.
const GATED_KIND_NAMES = new Set(
	Object.entries(KIND_GATED_BY_DEPLOY_RULES)
		.filter(([, gated]) => gated)
		.map(([kind]) => kind)
)

export function kindGatedByDeployRules(kind: Kind | UserDraftItemKind): boolean {
	return GATED_KIND_NAMES.has(kind)
}

/**
 * Narrow a workspace-level refusal to one item kind. Only the direct-deployment term is scoped:
 * the deployers-only term over-reaches the same way, but it does so on `main` too, and loosening
 * it here would change behaviour beyond mirroring the server.
 */
export function deployPermissionForKind(
	perm: DeployPermission,
	kind: Kind | UserDraftItemKind
): DeployPermission {
	if (perm.ok) return perm
	if (perm.refusedBy === 'DisableDirectDeployment' && !kindGatedByDeployRules(kind)) {
		return { ok: true }
	}
	return perm
}

/** The refusal a bulk action carries: it applies as soon as one selected kind is still refused. */
export function deployPermissionForKinds(
	perm: DeployPermission,
	kinds: (Kind | UserDraftItemKind)[]
): DeployPermission {
	if (perm.ok) return perm
	// An empty selection keeps the workspace-level refusal, which is then the only thing left
	// to say why the action is unavailable.
	if (kinds.length === 0) return perm
	return kinds.some((k) => !deployPermissionForKind(perm, k).ok) ? perm : { ok: true }
}

/**
 * Whether the current user may deploy into `workspace`. Mirrors `check_deploy_rules` in
 * windmill-common so the UI can disable the action with a reason instead of letting the
 * click 403: `DisableDirectDeployment` is evaluated before `RestrictDeployToDeployers`, so
 * the same message wins here as on the server when both block; admins and superadmins bypass
 * both rules, while `wm_deployers` members bypass only the latter.
 *
 * The operator refusal is not part of that mirror. The server refuses operators in the item
 * handlers instead, and for fewer kinds, so refusing them for everything here is deliberately
 * stricter than the server rather than a faithful copy of it.
 *
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
			return {
				ok: false,
				reason: "You're an operator in this workspace — operators can't deploy",
				refusedBy: 'operator'
			}
		}
		const userInfo = {
			is_admin: !!me.is_admin,
			is_super_admin: !!me.is_super_admin,
			username: me.username,
			groups: me.groups ?? []
		}
		// A superadmin who is only a plain member of the workspace still bypasses every rule,
		// so dropping either term here refuses a deploy the server accepts.
		if (!userInfo.is_admin && !userInfo.is_super_admin) {
			const rulesets = await fetchProtectionRulesForWorkspace(workspace)
			if (!canUserBypassRuleKindInRulesets(rulesets, 'DisableDirectDeployment', userInfo)) {
				// The reserved dev-workspace lock carries DisableWorkspaceForking alongside this rule,
				// so suggesting a fork unconditionally would point at a second blocked action.
				const canFork = canUserBypassRuleKindInRulesets(
					rulesets,
					'DisableWorkspaceForking',
					userInfo
				)
				const advice = canFork
					? 'fork the workspace or open a pull request'
					: 'make your changes locally and open a pull request'
				return {
					ok: false,
					reason: `Direct deployment to ${workspace} is disabled — ${advice}`,
					refusedBy: 'DisableDirectDeployment'
				}
			}
			// `wm_deployers` membership is an implicit pass on this rule only, so it cannot
			// short-circuit the fetch above the way admin does.
			const isDeployer = userInfo.groups.includes('wm_deployers')
			if (
				!isDeployer &&
				!canUserBypassRuleKindInRulesets(rulesets, 'RestrictDeployToDeployers', userInfo)
			) {
				return {
					ok: false,
					reason: `Only workspace admins and members of wm_deployers can deploy to ${workspace}`,
					refusedBy: 'RestrictDeployToDeployers'
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
 * `checkDeployPermission`'s workspace-level rules don't cover. Advisory only: it exists so the UI
 * can refuse with a reason instead of letting the write fail, and the server stays the enforcement
 * point. Same fail-open contract — every uncertain answer is `ok`.
 *
 * `me.folders` is the target workspace's *write* set, so a folder absent from it is one this user
 * can read at most. `folderExists` is injected so the decision can be exercised without a backend.
 */
export async function checkPathWritePermission(
	workspace: string,
	path: string,
	me: Pick<User, 'is_admin' | 'is_super_admin' | 'username' | 'folders'>,
	folderExists: (folderPath: string) => Promise<boolean> = (folderPath) =>
		checkItemExists('folder', folderPath, workspace)
): Promise<DeployPermission> {
	// The server's `is_owner` reads `ApiAuthed.is_admin`, the merged `is_admin || super_admin`,
	// so a superadmin owns every path here whether or not they own the folder.
	if (me.is_admin || me.is_super_admin) return { ok: true }
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
		if (!(await folderExists(`f/${folder}`))) return { ok: true }
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
		canPreserveOnBehalfOf:
			me.is_admin || me.is_super_admin || (me.groups ?? []).includes('wm_deployers'),
		me: { email: me.email, permissionedAs: `u/${me.username}` }
	}
}
