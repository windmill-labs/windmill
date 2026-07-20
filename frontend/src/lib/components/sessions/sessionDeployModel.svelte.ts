import { getDraftItems, type DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	checkDeployPermission,
	checkItemExists,
	getItemValue,
	type DeployPermission,
	type DeployResult
} from '$lib/utils_workspace_deploy'
import {
	deployDraft,
	discardDraft,
	fetchDraftBaseStale,
	getDraftDiffValues
} from '$lib/utils_draft_deploy'
import { untrack } from 'svelte'
import {
	buildDeployItems,
	deployPlanFor,
	diffBaseFor,
	discardPlanFor,
	maskOnlyCandidates,
	type DeployItem,
	type DeployPlanEntry
} from './sessionDeployModel'
import { maskKey } from './modifiedItemsMask'
import { sessionState } from './sessionState.svelte'
import { logFeatureUsage } from '$lib/utils/featureUsage'

export type DeploymentStatus = { status: 'loading' | 'failed'; error?: string }

// Reactive wrapper over the pure sessionDeployModel: owns the async draft list
// and builds the unified item list. The diff-value loader lives here too so both
// the tree and the diff column read one resolver. The dock is granular per-item
// deploy (deployRow/discardRow); batch flows and fork→parent promotion are the
// compare page's concern.

export interface SessionDeployModelArgs {
	workspaceId: string
	/** Called after a deploy/discard mutated workspace state. Lets the consumer
	 *  refresh ITS OWN data sources — without this, the bar shows stale counts
	 *  until the next turn end / tab refocus / reload. */
	onDataChanged?: () => void
	/** A draft deployed: the consumer owning the mask moves the item's entry to
	 *  its deployed path (`displayPath` — differs from the synthetic storage
	 *  `path` for draft-only items, which never exists deployed). */
	onItemDeployed?: (item: DeployItem) => void
	/** A draft was discarded: the chat's touch is undone — the consumer drops
	 *  the item's mask entry so a pre-existing deployed item doesn't keep
	 *  reading as this chat's "Deployed" edit. */
	onItemDiscarded?: (item: DeployItem) => void
	/** Chat-modified-items mask; undefined shows every draft. */
	mask?: Set<string>
}

export interface DiffValues {
	before: unknown
	after: unknown
}

/** Resolves which mask-only candidates still exist in the session workspace
 *  (deployed) vs are gone (discarded drafts / deleted items) — the
 *  `existingKeys` input of `buildDeployItems`. Undefined while resolving, so
 *  deployed rows only appear once confirmed. Shared by the drawer model and the
 *  bar's dock counts. */
export function useExistingMaskKeys(
	getInput: () => Omit<Parameters<typeof buildDeployItems>[0], 'existingKeys'> & {
		workspaceId: string
	}
) {
	let existingKeys = $state<Set<string> | undefined>(undefined)
	// Existence is checked once per distinct candidate set; the signature avoids
	// re-firing while results stream back.
	let lastCandidateSig = ''
	// Bumped by refresh() to force a re-check even when the candidate set (the
	// effect's other dependency) is unchanged.
	let recheck = $state(0)
	$effect(() => {
		void recheck
		const { workspaceId, ...buildInput } = getInput()
		const cands = maskOnlyCandidates(buildInput)
		// The workspace is part of the identity: the same candidate keys in a
		// different workspace must re-check, not reuse the previous verdict.
		const sig =
			workspaceId +
			'|' +
			cands
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
			// workspace; a discarded or deleted one doesn't.
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
		/** Force a fresh existence check (an item may have been deleted since).
		 *  Hard invalidate: current keys are dropped until the check resolves, so
		 *  rows derived from them disappear meanwhile — use on a fresh surface
		 *  (drawer open), not on a live one. */
		reset() {
			lastCandidateSig = ''
			existingKeys = undefined
		},
		/** Re-check while keeping the current keys (stale-while-revalidate), so a
		 *  live surface doesn't blank out during the round-trip. */
		refresh() {
			lastCandidateSig = ''
			recheck++
		}
	}
}

export function useSessionDeployModel(getArgs: () => SessionDeployModelArgs) {
	let draftItems = $state<DraftItem[]>([])
	let loading = $state(false)
	let error = $state<string | undefined>(undefined)

	// Mask keys confirmed to still exist → allowed to show as terminal deployed
	// rows (a discarded draft is a mask entry with no item, so it stays out).
	const existing = useExistingMaskKeys(() => ({
		draftItems,
		mask: getArgs().mask,
		workspaceId: getArgs().workspaceId
	}))

	// Optimistic knowledge from this model's own deploys: a deployed key exists
	// in the session workspace before the async existence check confirms it.
	// Without this a deployed row falls out of the list for ~1s, unmounting its
	// card.
	let deployedKeys = $state<ReadonlySet<string>>(new Set())

	const items = $derived.by(() => {
		const existingKeys =
			deployedKeys.size === 0 ? existing.keys : new Set([...(existing.keys ?? []), ...deployedKeys])
		return buildDeployItems({ draftItems, mask: getArgs().mask, existingKeys })
	})

	async function fetchDrafts() {
		loading = true
		error = undefined
		try {
			draftItems = await getDraftItems(getArgs().workspaceId)
		} catch (e) {
			console.error('Session deploy model: draft list failed', e)
			error = `Failed to load drafts: ${e}`
			draftItems = []
		} finally {
			loading = false
		}
	}

	/** (Re)fetch. Called on open. */
	function load() {
		// Re-check existence fresh (an item may have been deleted since last open).
		existing.reset()
		// Drop the optimistic bridge with it: it only covers the gap until the
		// fresh check resolves, and a stale entry would keep a since-deleted item
		// reading as deployed forever.
		deployedKeys = new Set()
		// Staleness can change between opens (a deploy elsewhere moves the head).
		staleFetched.clear()
		staleKeys = new Set()
		void fetchDrafts()
	}

	// ── Stale drafts (base ≠ deployed head) ──────────────────────────────────
	// A draft forked from a version that is no longer the deployed head: someone
	// deployed in between, so deploying the draft would silently revert them.
	// Same detection as the compare page; surfaced as a warning, deploy stays
	// allowed. Only own drafts on deployed script/flow/app items carry a base.
	let staleKeys = $state<ReadonlySet<string>>(new Set())
	const staleFetched = new Set<string>()
	$effect(() => {
		const list = items
		const ws = getArgs().workspaceId
		untrack(() => {
			for (const it of list) {
				if (!it.hasDraft || it.draftOnly || !it.mine) continue
				if (!['script', 'flow', 'app', 'raw_app'].includes(it.draftKind)) continue
				if (staleFetched.has(it.key)) continue
				staleFetched.add(it.key)
				void fetchDraftBaseStale(it.draftKind, it.path, ws).then((stale) => {
					if (stale) staleKeys = new Set(staleKeys).add(it.key)
				})
			}
		})
	})

	// ── Deploy permission ────────────────────────────────────────────────────
	// Preflight the shared checkDeployPermission (operator / RestrictDeployToDeployers)
	// for the session workspace so the button disables with a reason instead of
	// failing on click. `ok` defaults true while resolving (fail-open).
	let deployPerm = $state<DeployPermission>({ ok: true })
	let deployPermFetchedFor = ''
	$effect(() => {
		const ws = getArgs().workspaceId
		untrack(() => {
			if (!ws || deployPermFetchedFor === ws) return
			deployPermFetchedFor = ws
			// Reset to fail-open for the new workspace and drop a stale resolution
			// (a slower fetch for the previous workspace must not gate this one).
			deployPerm = { ok: true }
			void checkDeployPermission(ws).then((perm) => {
				if (deployPermFetchedFor === ws) deployPerm = perm
			})
		})
	})

	// ── Deploy execution ─────────────────────────────────────────────────────
	// Per-item transient deploy state (keyed by DeployItem.key): loading or
	// failed. Success has no status — the row's badge flipping to Deployed is
	// the feedback. Reassigned, not mutated.
	let deploymentStatus = $state<Record<string, DeploymentStatus>>({})
	let deploying = $state(false)

	function setStatus(key: string, s: DeploymentStatus | undefined) {
		const next = { ...deploymentStatus }
		if (s) next[key] = s
		else delete next[key]
		deploymentStatus = next
	}

	async function runPlan(entry: DeployPlanEntry): Promise<DeployResult> {
		const cur = getArgs().workspaceId
		switch (entry.op) {
			case 'deploy_draft':
				return deployDraft(entry.draftKind, entry.path, cur, {
					draftOnly: entry.draftOnly,
					rawApp: entry.rawApp
				})
			case 'discard':
				return discardDraft(entry.draftKind, entry.path, cur, entry.draftOnly, entry.legacy)
		}
	}

	function refreshData() {
		void fetchDrafts()
		getArgs().onDataChanged?.()
	}

	/** Deploy (or discard) a single item; returns whether it succeeded. */
	async function deployOne(item: DeployItem, discard = false): Promise<boolean> {
		const plan = discard ? discardPlanFor(item) : deployPlanFor(item)
		if (!plan) return false
		// Don't attempt a deploy we know the user can't make (no write permission
		// on the path, or blocked by the operator / deployer rule) — the UI
		// disables it too; this is the guard behind that.
		if (!discard && (!item.canWrite || !deployPerm.ok)) return false
		setStatus(item.key, { status: 'loading' })
		deploying = true
		try {
			const res = await runPlan(plan)
			setStatus(item.key, res.success ? undefined : { status: 'failed', error: res.error })
			if (res.success) {
				if (discard) {
					getArgs().onItemDiscarded?.(item)
				} else {
					// Bridge under the DEPLOYED path's key too: the mask rewrite
					// (onItemDeployed) re-keys a synthetic-storage row to displayPath.
					deployedKeys = new Set(deployedKeys)
						.add(item.key)
						.add(maskKey(item.draftKind, item.displayPath))
					getArgs().onItemDeployed?.(item)
					logFeatureUsage('ai_session', 'deployed', {
						key: item.draftKind,
						entityId: sessionState.currentSessionId,
						workspace: getArgs().workspaceId
					})
				}
			}
			return res.success
		} finally {
			deploying = false
		}
	}

	/** Returns success so the UI can stage its post-deploy animation. */
	async function deployRow(item: DeployItem): Promise<boolean> {
		const ok = await deployOne(item)
		if (ok) refreshData()
		return ok
	}

	async function discardRow(item: DeployItem) {
		if (await deployOne(item, true)) refreshData()
	}

	// ── Diff values (one resolver for tree + column) ─────────────────────────
	async function loadDiffValues(item: DeployItem): Promise<DiffValues> {
		const base = diffBaseFor(item, getArgs().workspaceId)
		if (base.kind === 'draft') {
			const { deployed, draft } = await getDraftDiffValues(
				base.draftKind,
				item.path,
				base.workspaceId,
				base.draftOnly
			)
			return { before: base.draftOnly ? undefined : deployed, after: draft }
		}
		const v = await getItemValue(base.deployKind, base.path, base.workspaceId).catch(
			() => undefined
		)
		return { before: v, after: v }
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
		load,
		loadDiffValues,
		get deploying() {
			return deploying
		},
		statusOf(key: string): DeploymentStatus | undefined {
			return deploymentStatus[key]
		},
		/** Whether a draft row's base is stale (deployed head moved since). */
		staleOf(key: string): boolean {
			return staleKeys.has(key)
		},
		/** Whether the user may deploy into the session workspace. */
		get deployPermission(): DeployPermission {
			return deployPerm
		},
		deployRow,
		discardRow
	}
}

export type SessionDeployModel = ReturnType<typeof useSessionDeployModel>
