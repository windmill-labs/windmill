import { UserService, WorkspaceService, type WorkspaceComparison } from '$lib/gen'
import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	deleteItemInWorkspace,
	deployItem,
	getItemValue,
	getOnBehalfOf,
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
	actionFor,
	buildDeployItems,
	defaultSelection,
	deployPlanFor,
	diffBaseFor,
	discardPlanFor,
	footerSummary,
	isOnBehalfEligible,
	itemsInSegment,
	segmentCounts,
	selectableOf,
	type DeployItem,
	type DeployPlanEntry,
	type DeploySegment,
	type FooterSummary,
	type SessionContext
} from './sessionDeployModel'

export type DeploymentStatus = { status: 'loading' | 'deployed' | 'failed'; error?: string }

// Reactive wrapper over the pure sessionDeployModel: owns the two async data
// sources (per-user draft list + fork comparison), builds the unified item list,
// and holds the drawer's selection + active-segment state. The diff-value loader
// lives here too so both the tree and the diff column read one resolver (it
// replaces SessionDiffDrawer's inline loadUnifiedValues/loadDraftValues).
//
// S2: read + selection only. Deploy execution (deployPlanFor → deployItem/
// deployDraft/…) and on-behalf/behind gating land in S3/S4.

export interface SessionDeployModelArgs {
	workspaceId: string
	parentWorkspaceId?: string
	parentName?: string
	isFork: boolean
	/** Chat-modified-items mask; undefined shows every draft/diff. */
	mask?: Set<string>
}

export interface DiffValues {
	before: unknown
	after: unknown
}

export function useSessionDeployModel(getArgs: () => SessionDeployModelArgs) {
	let draftItems = $state<DraftItem[]>([])
	let comparison = $state<WorkspaceComparison | undefined>(undefined)
	let draftLoading = $state(false)
	let forkLoading = $state(false)
	let draftError = $state<string | undefined>(undefined)
	let forkError = $state<string | undefined>(undefined)

	// Selection is a plain key Set; reassigned (not mutated) so Svelte tracks it.
	let selectedKeys = $state<Set<string>>(new Set())
	let segment = $state<DeploySegment>('to_review')
	// One-shot default selection: don't re-check rows the user deselected after a
	// later refetch. Reset by load() so reopening re-defaults.
	let autoSelected = false

	const context = $derived<SessionContext>({
		isFork: getArgs().isFork,
		currentWorkspaceId: getArgs().workspaceId,
		parentWorkspaceId: getArgs().parentWorkspaceId,
		parentName: getArgs().parentName
	})

	const items = $derived(
		buildDeployItems({ draftItems, comparison, mask: getArgs().mask, context })
	)
	const counts = $derived(segmentCounts(items))
	const visibleItems = $derived(itemsInSegment(items, segment))
	const footer = $derived(footerSummary(items, selectedKeys, context))

	const loading = $derived(context.isFork ? draftLoading || forkLoading : draftLoading)
	const error = $derived(context.isFork ? (forkError ?? draftError) : draftError)
	const notice = $derived(
		context.isFork && comparison?.skipped_comparison
			? 'This fork was created before change tracking was added — diffs are not available.'
			: undefined
	)

	// Seed the default selection (all selectable) once the first item set loads.
	$effect(() => {
		if (autoSelected || items.length === 0) return
		selectedKeys = new Set(defaultSelection(items))
		autoSelected = true
	})

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

	/** (Re)fetch both sources and re-arm the default selection. Called on open. */
	function load() {
		autoSelected = false
		void fetchDrafts()
		if (getArgs().isFork) void fetchComparison()
	}

	// ── Selection ──────────────────────────────────────────────────────────
	function isSelected(key: string): boolean {
		return selectedKeys.has(key)
	}
	function setSelected(keys: string[], on: boolean) {
		const next = new Set(selectedKeys)
		for (const k of keys) {
			if (on) next.add(k)
			else next.delete(k)
		}
		selectedKeys = next
	}
	function toggle(key: string) {
		setSelected([key], !selectedKeys.has(key))
	}
	function clearSelection() {
		selectedKeys = new Set()
	}
	/** Selectable keys among a set of items (drives select-all / subtree checks). */
	function selectableKeysOf(list: DeployItem[]): string[] {
		return list.filter(selectableOf).map((i) => i.key)
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
		if (item.state !== 'in_fork' || item.ahead <= 0) return false
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
				if (it.state !== 'in_fork' || !isOnBehalfEligible(it.deployKind)) continue
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

	const hasUnselectedOnBehalf = $derived(
		items.some((i) => selectedKeys.has(i.key) && needsOnBehalfChoice(i))
	)

	// ── Deploy execution ─────────────────────────────────────────────────────
	// Per-item deploy state (keyed by DeployItem.key). Reassigned, not mutated.
	let deploymentStatus = $state<Record<string, DeploymentStatus>>({})
	let deploying = $state(false)
	// Fork comparison recomputes asynchronously (~hundreds of ms) after a deploy,
	// so an immediate re-fetch returns the pre-change tally — re-poll to catch up.
	let pollTimers: ReturnType<typeof setTimeout>[] = []

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
			case 'update_fork':
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
	}

	/** Deploy (or discard) a single item; returns whether it succeeded. */
	async function deployOne(item: DeployItem, discard = false): Promise<boolean> {
		const plan = discard ? discardPlanFor(item, context) : deployPlanFor(item, context)
		if (!plan) return false
		// Never promote an eligible item to the parent without a resolved identity.
		if (!discard && needsOnBehalfChoice(item)) return false
		setStatus(item.key, { status: 'loading' })
		const res = await runPlan(plan, discard ? undefined : resolveOnBehalf(item))
		setStatus(
			item.key,
			res.success ? { status: 'deployed' } : { status: 'failed', error: res.error }
		)
		return res.success
	}

	/** Deploy all selected, actionable rows targeting a given stage. Folders first
	 *  (a folder must exist before items land under it), deployed rows skipped. */
	async function deploySelected(targetStage: 'fork' | 'parent') {
		if (deploying) return
		deploying = true
		try {
			const targets = items
				.filter((i) => selectedKeys.has(i.key))
				.filter((i) => deploymentStatus[i.key]?.status !== 'deployed')
				.filter((i) => {
					const a = actionFor(i, context)
					return a.op !== 'none' && a.targetStage === targetStage
				})
				.sort((a, b) => (a.deployKind === 'folder' ? -1 : 0) - (b.deployKind === 'folder' ? -1 : 0))
			const deployed: string[] = []
			for (const item of targets) {
				if (await deployOne(item)) deployed.push(item.key)
			}
			if (deployed.length > 0) {
				setSelected(deployed, false)
				refreshData()
			}
		} finally {
			deploying = false
		}
	}

	async function deployRow(item: DeployItem) {
		if (await deployOne(item)) {
			setSelected([item.key], false)
			refreshData()
		}
	}

	async function discardRow(item: DeployItem) {
		if (await deployOne(item, true)) {
			setSelected([item.key], false)
			refreshData()
		}
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
		return { before: undefined, after: undefined }
	}

	return {
		get items() {
			return items
		},
		get visibleItems() {
			return visibleItems
		},
		get counts() {
			return counts
		},
		get footer(): FooterSummary {
			return footer
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
		get segment() {
			return segment
		},
		setSegment(s: DeploySegment) {
			segment = s
		},
		isSelected,
		toggle,
		setSelected,
		clearSelection,
		selectableKeysOf,
		load,
		loadDiffValues,
		get deploying() {
			return deploying
		},
		statusOf(key: string): DeploymentStatus | undefined {
			return deploymentStatus[key]
		},
		deployRow,
		discardRow,
		deploySelected,
		// On-behalf-of
		get hasUnselectedOnBehalf() {
			return hasUnselectedOnBehalf
		},
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
