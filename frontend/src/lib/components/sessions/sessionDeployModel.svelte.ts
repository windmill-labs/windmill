import { UserService, WorkspaceService, type WorkspaceComparison } from '$lib/gen'
import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	checkDeployPermission,
	checkItemExists,
	deleteItemInWorkspace,
	deployItem,
	getItemValue,
	getOnBehalfOf,
	type DeployPermission,
	type DeployResult
} from '$lib/utils_workspace_deploy'
import { deployDraft, discardDraft, getDraftDiffValues } from '$lib/utils_draft_deploy'
import { isTriggerOrScheduleKind } from 'windmill-utils-internal'
import { untrack } from 'svelte'
import {
	needsOnBehalfOfSelection,
	type OnBehalfOfChoice,
	type OnBehalfOfDetails
} from '../OnBehalfOfSelector.svelte'
import {
	buildDeployItems,
	deployPlanFor,
	diffBaseFor,
	discardPlanFor,
	isOnBehalfEligible,
	maskOnlyCandidates,
	type DeployItem,
	type DeployPlanEntry,
	type SessionContext
} from './sessionDeployModel'

export type DeploymentStatus = { status: 'loading' | 'deployed' | 'failed'; error?: string }

// Reactive wrapper over the pure sessionDeployModel: owns the two async data
// sources (per-user draft list + fork comparison) and builds the unified item
// list. The diff-value loader lives here too so both the tree and the diff column
// read one resolver. The dock is granular per-item deploy (deployRow/discardRow);
// there is no in-dock batch/selection — that moved to the compare page.

export interface SessionDeployModelArgs {
	workspaceId: string
	/** Display name of the (fork) workspace, for the pipeline's middle dot. */
	workspaceName?: string
	parentWorkspaceId?: string
	parentName?: string
	/** Called after a deploy/discard mutated workspace state. Lets the consumer
	 *  refresh ITS OWN data sources (the session bar caches the fork comparison
	 *  on the runtime) — without this, the bar shows stale counts until the next
	 *  turn end / tab refocus / reload. */
	onDataChanged?: () => void
	isFork: boolean
	/** Chat-modified-items mask; undefined shows every draft/diff. */
	mask?: Set<string>
}

export interface DiffValues {
	before: unknown
	after: unknown
}

/** Resolves which mask-only candidates still exist in the session workspace
 *  (deployed) vs are gone (discarded drafts) — the `existingKeys` input of
 *  `buildDeployItems`. Undefined while resolving, so terminal rows only appear
 *  once confirmed. Shared by the drawer model and the bar's dock counts. */
export function useExistingMaskKeys(
	getInput: () => Omit<Parameters<typeof buildDeployItems>[0], 'existingKeys'> & {
		workspaceId: string
	}
) {
	let existingKeys = $state<Set<string> | undefined>(undefined)
	// Existence is checked once per distinct candidate set; the signature avoids
	// re-firing while results stream back.
	let lastCandidateSig = ''
	$effect(() => {
		const { workspaceId, ...buildInput } = getInput()
		const cands = maskOnlyCandidates(buildInput)
		const sig = cands
			.map((c) => c.key)
			.sort()
			.join(',')
		untrack(() => {
			if (sig === lastCandidateSig) return
			lastCandidateSig = sig
			if (cands.length === 0) {
				existingKeys = new Set()
				return
			}
			// A mask-only item that was deployed still exists in the session
			// workspace (it was deployed here first); a discarded one doesn't.
			void Promise.all(
				cands.map((c) =>
					checkItemExists(c.deployKind, c.path, workspaceId)
						.then((exists) => (exists ? c.key : null))
						.catch(() => null)
				)
			).then((keys) => {
				// A slower batch for a superseded candidate set (or one voided by
				// reset()) must not overwrite the current result.
				if (sig !== lastCandidateSig) return
				existingKeys = new Set(keys.filter((k): k is string => !!k))
			})
		})
	})
	return {
		get keys() {
			return existingKeys
		},
		/** Force a fresh existence check (an item may have been deleted since). */
		reset() {
			lastCandidateSig = ''
			existingKeys = undefined
		}
	}
}

export function useSessionDeployModel(getArgs: () => SessionDeployModelArgs) {
	let draftItems = $state<DraftItem[]>([])
	let comparison = $state<WorkspaceComparison | undefined>(undefined)
	let draftLoading = $state(false)
	let forkLoading = $state(false)
	let draftError = $state<string | undefined>(undefined)
	let forkError = $state<string | undefined>(undefined)

	const context = $derived<SessionContext>({
		isFork: getArgs().isFork,
		currentWorkspaceId: getArgs().workspaceId,
		currentName: getArgs().workspaceName,
		parentWorkspaceId: getArgs().parentWorkspaceId,
		parentName: getArgs().parentName
	})

	// Mask keys confirmed to still exist → allowed to show as terminal "In parent"
	// rows (a discarded draft is a mask entry with no item, so it stays out).
	const existing = useExistingMaskKeys(() => ({
		draftItems,
		comparison,
		mask: getArgs().mask,
		context,
		workspaceId: getArgs().workspaceId
	}))

	const items = $derived(
		buildDeployItems({
			draftItems,
			comparison,
			mask: getArgs().mask,
			existingKeys: existing.keys,
			context
		})
	)

	const loading = $derived(context.isFork ? draftLoading || forkLoading : draftLoading)
	const error = $derived(context.isFork ? (forkError ?? draftError) : draftError)
	const notice = $derived(
		context.isFork && comparison?.skipped_comparison
			? 'This fork was created before change tracking was added — diffs are not available.'
			: undefined
	)

	async function fetchDrafts() {
		draftLoading = true
		draftError = undefined
		try {
			draftItems = await getDraftItems(getArgs().workspaceId)
		} catch (e) {
			console.error('Session deploy model: draft list failed', e)
			draftError = `Failed to load drafts: ${e}`
			draftItems = []
		} finally {
			draftLoading = false
		}
	}

	async function fetchComparison() {
		const { parentWorkspaceId, workspaceId } = getArgs()
		if (!parentWorkspaceId) return
		forkLoading = true
		forkError = undefined
		try {
			comparison = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: workspaceId
			})
		} catch (e) {
			console.error('Session deploy model: comparison failed', e)
			forkError = `Failed to load comparison: ${e}`
			comparison = undefined
		} finally {
			forkLoading = false
		}
	}

	/** (Re)fetch both sources. Called on open. */
	function load() {
		// Re-check existence fresh (an item may have been deleted since last open).
		existing.reset()
		void fetchDrafts()
		if (getArgs().isFork) void fetchComparison()
	}

	// ── On-behalf-of (fork→parent promotions of runnables/triggers) ──────────
	// An item carries a "run on behalf of" identity in the parent. When the source
	// (fork) item has one set, the user must choose whose identity the deployed
	// item runs as; the deploy button is gated until they do. Ported from
	// CompareWorkspaces. Only the deploy-to-parent direction is modelled (the
	// session pipeline never promotes fork→parent in reverse here).
	let onBehalfInfo = $state<Record<string, string | undefined>>({})
	let onBehalfChoice = $state<Record<string, OnBehalfOfChoice>>({})
	let customOnBehalf = $state<Record<string, OnBehalfOfDetails>>({})
	let canPreserveInParent = $state(false)
	// Non-reactive fetch guard: a stored `undefined` value (no identity) must not
	// look "unfetched" and re-trigger the request.
	const onBehalfFetched = new Set<string>()

	function obKey(ws: string, key: string): string {
		return `${ws}/${key}`
	}
	function sourceOnBehalf(item: DeployItem): string | undefined {
		return onBehalfInfo[obKey(getArgs().workspaceId, item.key)]
	}
	function targetOnBehalf(item: DeployItem): string | undefined {
		const parent = getArgs().parentWorkspaceId
		return parent ? onBehalfInfo[obKey(parent, item.key)] : undefined
	}
	/** Only fork→parent promotions of eligible kinds whose source has an identity
	 *  set, and only until the user has picked. */
	function needsOnBehalfChoice(item: DeployItem): boolean {
		if (item.parent !== 'ahead' || item.local) return false
		if (!isOnBehalfEligible(item.deployKind)) return false
		return (
			needsOnBehalfOfSelection(item.deployKind, sourceOnBehalf(item)) &&
			onBehalfChoice[item.key] === undefined
		)
	}
	function resolveOnBehalf(item: DeployItem): string | undefined {
		const choice = onBehalfChoice[item.key]
		if (choice === 'target') return targetOnBehalf(item)
		if (choice === 'custom') {
			const d = customOnBehalf[item.key]
			return isTriggerOrScheduleKind(item.deployKind) ? d?.permissionedAs : d?.email
		}
		// 'me' / undefined → deploying user's identity (backend default).
		return undefined
	}
	function setOnBehalfChoice(key: string, choice: OnBehalfOfChoice, details?: OnBehalfOfDetails) {
		onBehalfChoice = { ...onBehalfChoice, [key]: choice }
		if (details) customOnBehalf = { ...customOnBehalf, [key]: details }
	}

	// Fetch source+target on-behalf identities for eligible in-fork items.
	$effect(() => {
		const list = items
		const parent = getArgs().parentWorkspaceId
		const cur = getArgs().workspaceId
		if (!parent) return
		untrack(() => {
			for (const it of list) {
				if (it.parent !== 'ahead' || it.local || !isOnBehalfEligible(it.deployKind)) continue
				for (const ws of [cur, parent]) {
					const wk = obKey(ws, it.key)
					if (onBehalfFetched.has(wk)) continue
					onBehalfFetched.add(wk)
					void getOnBehalfOf(it.deployKind, it.path, ws)
						.then((v) => (onBehalfInfo = { ...onBehalfInfo, [wk]: v }))
						.catch(() => (onBehalfInfo = { ...onBehalfInfo, [wk]: undefined }))
				}
			}
		})
	})

	// Whether the user may preserve an existing on-behalf identity in the parent
	// (admin / wm_deployers) — gates the selector's "target" option.
	$effect(() => {
		const parent = getArgs().parentWorkspaceId
		if (!parent) return
		untrack(() => {
			void UserService.whoami({ workspace: parent })
				.then(
					(u) => (canPreserveInParent = u.is_admin || u.groups?.includes('wm_deployers') || false)
				)
				.catch(() => (canPreserveInParent = false))
		})
	})

	// ── Deploy permission per target workspace (fork + parent) ───────────────
	// Preflight the shared checkDeployPermission (operator / RestrictDeployToDeployers)
	// for each target workspace so the button disables with a reason instead of
	// failing on click. `ok` defaults true while resolving (fail-open).
	let deployPerms = $state<Record<string, DeployPermission>>({})
	const deployPermFetched = new Set<string>()

	$effect(() => {
		const cur = getArgs().workspaceId
		const parent = getArgs().parentWorkspaceId
		untrack(() => {
			for (const ws of parent ? [cur, parent] : [cur]) {
				if (!ws || deployPermFetched.has(ws)) continue
				deployPermFetched.add(ws)
				void checkDeployPermission(ws).then(
					(perm) => (deployPerms = { ...deployPerms, [ws]: perm })
				)
			}
		})
	})

	function deployPermission(ws: string | undefined): DeployPermission {
		return (ws && deployPerms[ws]) || { ok: true }
	}

	// ── Deploy execution ─────────────────────────────────────────────────────
	// Per-item deploy state (keyed by DeployItem.key). Reassigned, not mutated.
	let deploymentStatus = $state<Record<string, DeploymentStatus>>({})
	let deploying = $state(false)
	// Fork comparison recomputes asynchronously (~hundreds of ms) after a deploy,
	// so an immediate re-fetch returns the pre-change tally — re-poll to catch up.
	let pollTimers: ReturnType<typeof setTimeout>[] = []
	// The re-polls must die with the consuming component: a drawer closed right
	// after a deploy must not keep hitting compareWorkspaces and writing into
	// discarded state.
	$effect(() => {
		return () => pollTimers.forEach(clearTimeout)
	})

	function setStatus(key: string, s: DeploymentStatus | undefined) {
		const next = { ...deploymentStatus }
		if (s) next[key] = s
		else delete next[key]
		deploymentStatus = next
	}

	async function runPlan(entry: DeployPlanEntry, onBehalfOf?: string): Promise<DeployResult> {
		const cur = getArgs().workspaceId
		switch (entry.op) {
			case 'deploy_draft':
				return deployDraft(entry.draftKind!, entry.path, cur, {
					draftOnly: entry.draftOnly,
					rawApp: entry.rawApp
				})
			case 'discard':
				return discardDraft(entry.draftKind!, entry.path, cur, entry.draftOnly, entry.legacy)
			case 'deploy_item':
				return deployItem({
					kind: entry.deployKind,
					path: entry.path,
					workspaceFrom: entry.workspaceFrom!,
					workspaceTo: entry.workspaceTo!,
					onBehalfOf
				})
			case 'delete_in_parent':
				return deleteItemInWorkspace(entry.deployKind, entry.path, entry.workspaceTo!)
		}
	}

	function refreshData() {
		void fetchDrafts()
		if (getArgs().isFork) {
			pollTimers.forEach(clearTimeout)
			void fetchComparison()
			pollTimers = [800, 1800, 3500].map((d) => setTimeout(() => void fetchComparison(), d))
		}
		getArgs().onDataChanged?.()
	}

	/** The workspace a plan writes into — the current workspace for a draft deploy
	 *  (deployed in place), the target workspace otherwise. */
	function planTargetWorkspace(entry: DeployPlanEntry): string | undefined {
		return entry.op === 'deploy_draft' ? getArgs().workspaceId : entry.workspaceTo
	}

	/** Deploy (or discard) a single item; returns whether it succeeded. */
	async function deployOne(item: DeployItem, discard = false): Promise<boolean> {
		const plan = discard ? discardPlanFor(item, context) : deployPlanFor(item, context)
		if (!plan) return false
		// Never promote an eligible item to the parent without a resolved identity.
		if (!discard && needsOnBehalfChoice(item)) return false
		// Don't attempt a deploy we know the user can't make (operator / deployer
		// rule) — the UI disables it too; this is the guard behind that.
		if (!discard && !deployPermission(planTargetWorkspace(plan)).ok) return false
		setStatus(item.key, { status: 'loading' })
		deploying = true
		try {
			const res = await runPlan(plan, discard ? undefined : resolveOnBehalf(item))
			setStatus(
				item.key,
				res.success ? { status: 'deployed' } : { status: 'failed', error: res.error }
			)
			return res.success
		} finally {
			deploying = false
		}
	}

	async function deployRow(item: DeployItem) {
		if (await deployOne(item)) refreshData()
	}

	async function discardRow(item: DeployItem) {
		if (await deployOne(item, true)) refreshData()
	}

	// ── Diff values (one resolver for tree + column) ─────────────────────────
	async function loadDiffValues(item: DeployItem): Promise<DiffValues> {
		const base = diffBaseFor(item, context)
		if (base.kind === 'draft') {
			const { deployed, draft } = await getDraftDiffValues(
				base.draftKind,
				item.path,
				base.workspaceId,
				base.draftOnly
			)
			return { before: base.draftOnly ? undefined : deployed, after: draft }
		}
		if (base.kind === 'fork_parent') {
			const [before, after] = await Promise.all([
				base.existsInParent
					? getItemValue(base.deployKind, base.path, base.parentWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined),
				base.existsInFork
					? getItemValue(base.deployKind, base.path, base.forkWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined)
			])
			return { before, after }
		}
		if (base.kind === 'self') {
			const v = await getItemValue(base.deployKind, base.path, base.workspaceId).catch(
				() => undefined
			)
			return { before: v, after: v }
		}
		return { before: undefined, after: undefined }
	}

	return {
		get items() {
			return items
		},
		get loading() {
			return loading
		},
		get error() {
			return error
		},
		get notice() {
			return notice
		},
		get context() {
			return context
		},
		load,
		loadDiffValues,
		get deploying() {
			return deploying
		},
		statusOf(key: string): DeploymentStatus | undefined {
			return deploymentStatus[key]
		},
		/** Whether the user may deploy into a workspace (fork or parent). */
		deployPermission,
		deployRow,
		discardRow,
		// On-behalf-of
		get canPreserveOnBehalf() {
			return canPreserveInParent
		},
		needsOnBehalfChoice,
		onBehalfChoiceOf(key: string): OnBehalfOfChoice {
			return onBehalfChoice[key]
		},
		targetOnBehalf,
		setOnBehalfChoice
	}
}

export type SessionDeployModel = ReturnType<typeof useSessionDeployModel>
