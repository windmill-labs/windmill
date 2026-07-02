import type { UserDraftItemKind, WorkspaceComparison } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import type { Kind } from '$lib/utils_deployable'
import { isTriggerOrScheduleKind } from 'windmill-utils-internal'
import { maskKey, forkDiffKindToUserDraftKind } from './modifiedItemsMask'

// Unified item model for the session Review & Deploy surface. This is the pure,
// UI-free core: it merges the two data sources the compare page keeps apart —
// the per-user draft list (Deployed ↔ draft) and the fork comparison
// (fork ↔ parent) — into ONE list of items, each carrying two independent axes
// (`local` = pending draft, `parent` = sync/ahead/conflict) plus a terminal
// `done` flag, from which the badge / action / pipeline / deploy plan are derived
// as pure functions the drawer renders and executes. The dock is session-scoped
// and granular (per-item next step); whole-fork drift and batch/PR flows are the
// compare page's concern, not modeled here.
//
// Kept free of Svelte runes and async so it unit-tests in the node project. The
// reactive wrapper (data loading, selection $state, deploy execution) lives in
// the sibling `.svelte.ts` and consumes these functions.

// ── Item taxonomy bridge ────────────────────────────────────────────────────
// The draft list speaks UserDraftItemKind (`trigger_schedule`, `raw_app`); the
// fork diff and deploy utils speak the layout `Kind` (`schedule`, `raw_app`,
// `http_trigger`). `forkDiffKindToUserDraftKind` (in modifiedItemsMask) bridges
// diff→draft; this is the inverse draft→deploy needed to key deploy calls and
// row icons. Only non-identity cases listed; everything else is identity.
const DEPLOY_KIND_BY_DRAFT_KIND: Partial<Record<UserDraftItemKind, Kind>> = {
	trigger_schedule: 'schedule',
	trigger_http: 'http_trigger',
	trigger_websocket: 'websocket_trigger',
	trigger_kafka: 'kafka_trigger',
	trigger_nats: 'nats_trigger',
	trigger_postgres: 'postgres_trigger',
	trigger_mqtt: 'mqtt_trigger',
	trigger_sqs: 'sqs_trigger',
	trigger_gcp: 'gcp_trigger',
	trigger_azure: 'azure_trigger',
	trigger_default_email: 'email_trigger',
	raw_app: 'raw_app'
}

function deployKindOf(draftKind: UserDraftItemKind, rawApp: boolean): Kind {
	if (rawApp) return 'raw_app'
	return DEPLOY_KIND_BY_DRAFT_KIND[draftKind] ?? (draftKind as unknown as Kind)
}

// ── Public types ────────────────────────────────────────────────────────────

/** How the item's deployed-in-fork version compares to the parent — independent
 *  of `local` (a pending draft). At session scope only ahead/conflict/sync occur:
 *  a pure-behind item is fork drift the session never touched (surfaced as a
 *  banner pointer to the compare page, not listed here). */
export type ParentAxis = 'sync' | 'ahead' | 'conflict'

export interface SessionContext {
	/** Fork (Draft→Fork→Parent) vs main (Draft→Parent, no fork stage). */
	isFork: boolean
	currentWorkspaceId: string
	/** Display name of the current (fork) workspace — the pipeline's middle dot. */
	currentName?: string
	parentWorkspaceId?: string
	/** Parent workspace display name, interpolated into deploy labels
	 *  ("Deploy to test-ai-sessions"). Falls back to "parent". */
	parentName?: string
}

export interface DeployItem {
	/** Canonical identity `${UserDraftItemKind}:${path}` — unifies the draft-list
	 *  and fork-diff kind taxonomies so a drafted-and-deployed item appears once. */
	key: string
	/** Layout/deploy kind (script, flow, raw_app, http_trigger, schedule…). */
	deployKind: Kind
	/** UserDraftItemKind for draft ops; present whenever the item has a draft. */
	draftKind?: UserDraftItemKind
	path: string
	/** Friendly path for display; storage `path` stays the op/edit key. */
	displayPath: string
	summary?: string
	/** Uncommitted draft in the fork (alias of `hasDraft`, the `local` axis). */
	local: boolean
	/** Deployed-in-fork vs parent; `sync` for draft-only / already-deployed rows.
	 *  `conflict` when the fork and parent both moved the item. */
	parent: ParentAxis
	/** Reached the parent — the terminal, already-deployed mask-only rows. */
	done: boolean
	/** Pending deletion: present in the parent, removed in the fork. */
	removed: boolean
	hasDraft: boolean
	draftOnly: boolean
	draftUsers?: { username?: string | null }[]
	/** Own draft (deployable/discardable). Non-draft fork rows default true. */
	mine: boolean
	canWrite: boolean
	/** Legacy workspace-level draft (drives discard's `legacy` flag). */
	legacy: boolean
	rawApp: boolean
	existsInParent: boolean
	existsInFork: boolean
	ahead: number
	behind: number
}

export interface BuildInput {
	draftItems: DraftItem[]
	comparison?: WorkspaceComparison
	/** Chat-modified-items mask (`UserDraftItemKind:path`). When set, only masked
	 *  items are included, and mask-only entries (no draft, no diff) surface as
	 *  terminal `in_parent` rows — but only when confirmed to still exist (see
	 *  `existingKeys`). */
	mask?: Set<string>
	/** Mask keys confirmed to still exist in the workspace (deployed). A mask-only
	 *  key that isn't here is treated as gone (a discarded draft) and dropped, so a
	 *  discarded item doesn't masquerade as "in parent". Undefined → no mask-only
	 *  rows are emitted yet (existence still resolving); callers that can't resolve
	 *  existence simply won't show terminal rows. */
	existingKeys?: Set<string>
	context: SessionContext
}

// ── Build: merge draft list + fork comparison into unified items ─────────────

/** The parent-comparison axis from a fork diff's ahead/behind counts. Draft-only
 *  and already-deployed rows (no fork diff) are `sync`. A behind-only row (parent
 *  moved, fork didn't) falls to `sync`/bare — it can't occur for a session-edited
 *  item, and is fork drift the dock leaves to the compare page either way. */
function parentAxisFrom(ahead: number, behind: number): ParentAxis {
	if (ahead > 0 && behind > 0) return 'conflict'
	if (ahead > 0) return 'ahead'
	return 'sync'
}

export function buildDeployItems(input: BuildInput): DeployItem[] {
	const { draftItems, comparison, mask, existingKeys, context } = input
	const scopedDrafts = mask
		? draftItems.filter((it) => mask.has(maskKey(it.kind, it.path)))
		: draftItems
	const draftByCanonical = new Map(scopedDrafts.map((it) => [maskKey(it.kind, it.path), it]))

	const out: DeployItem[] = []
	const seen = new Set<string>()

	// 1. Fork context only: items whose deployed fork version differs from parent.
	if (context.isFork) {
		for (const d of comparison?.diffs ?? []) {
			if (!d.has_changes) continue
			const udk = forkDiffKindToUserDraftKind(d.kind)
			if (!udk) continue // resource_type / folder — never AI-authored, skip
			const canonical = maskKey(udk, d.path)
			if (mask && !mask.has(canonical)) continue
			seen.add(canonical)
			const draft = draftByCanonical.get(canonical)
			const existsInFork = d.exists_in_fork !== false
			const existsInParent = d.exists_in_source !== false
			out.push({
				key: canonical,
				deployKind: d.kind as Kind,
				draftKind: draft?.kind,
				path: d.path,
				displayPath: draft?.draft_path ?? d.path,
				summary: draft?.summary,
				local: !!draft,
				parent: parentAxisFrom(d.ahead, d.behind),
				done: false,
				removed: existsInParent && !existsInFork,
				hasDraft: !!draft,
				draftOnly: draft?.draft_only ?? false,
				draftUsers: draft?.draft_users,
				mine: draft?.mine ?? true,
				canWrite: draft?.can_write ?? true,
				legacy: draft?.legacy_draft ?? false,
				rawApp: draft?.raw_app ?? d.kind === 'raw_app',
				existsInParent,
				existsInFork,
				ahead: d.ahead,
				behind: d.behind
			})
		}
	}

	// 2. Drafts with no fork diff (never deployed, or deployed-identical until this
	//    draft), and — in main context — every draft.
	for (const it of scopedDrafts) {
		const canonical = maskKey(it.kind, it.path)
		if (seen.has(canonical)) continue
		seen.add(canonical)
		out.push({
			key: canonical,
			deployKind: deployKindOf(it.kind, it.raw_app),
			draftKind: it.kind,
			path: it.path,
			displayPath: it.draft_path ?? it.path,
			summary: it.summary,
			local: true,
			parent: 'sync',
			done: false,
			removed: false,
			hasDraft: true,
			draftOnly: it.draft_only,
			draftUsers: it.draft_users,
			mine: it.mine,
			canWrite: it.can_write,
			legacy: it.legacy_draft,
			rawApp: it.raw_app,
			existsInParent: !it.draft_only,
			existsInFork: !it.draft_only,
			ahead: 0,
			behind: 0
		})
	}

	// 3. Mask-only items (touched by the chat, no draft, no diff) → terminal
	//    `in_parent` rows, but ONLY when confirmed to still exist. A discarded
	//    draft leaves a mask entry with no item; without this guard it would
	//    masquerade as "in parent".
	if (mask) {
		for (const k of mask) {
			if (seen.has(k)) continue
			if (!existingKeys?.has(k)) continue
			const sep = k.indexOf(':')
			if (sep < 0) continue
			const udk = k.slice(0, sep) as UserDraftItemKind
			const path = k.slice(sep + 1)
			seen.add(k)
			out.push({
				key: k,
				deployKind: deployKindOf(udk, udk === 'raw_app'),
				draftKind: udk,
				path,
				displayPath: path,
				summary: undefined,
				local: false,
				parent: 'sync',
				done: true,
				removed: false,
				hasDraft: false,
				draftOnly: false,
				draftUsers: undefined,
				mine: true,
				canWrite: true,
				legacy: false,
				rawApp: udk === 'raw_app',
				existsInParent: true,
				existsInFork: true,
				ahead: 0,
				behind: 0
			})
		}
	}

	return out
}

/** Mask keys not covered by a current draft or fork diff — the candidates for a
 *  terminal `in_parent` row. The reactive layer existence-checks these (in the
 *  session workspace) to tell a deployed item from a discarded one, then feeds
 *  the survivors back as `existingKeys`. */
export function maskOnlyCandidates(
	input: Omit<BuildInput, 'existingKeys'>
): { key: string; deployKind: Kind; draftKind: UserDraftItemKind; path: string }[] {
	const { draftItems, comparison, mask, context } = input
	if (!mask) return []
	const covered = new Set<string>()
	for (const it of draftItems) {
		const k = maskKey(it.kind, it.path)
		if (mask.has(k)) covered.add(k)
	}
	if (context.isFork) {
		for (const d of comparison?.diffs ?? []) {
			if (!d.has_changes) continue
			const udk = forkDiffKindToUserDraftKind(d.kind)
			if (!udk) continue
			const c = maskKey(udk, d.path)
			if (mask.has(c)) covered.add(c)
		}
	}
	const out: { key: string; deployKind: Kind; draftKind: UserDraftItemKind; path: string }[] = []
	for (const k of mask) {
		if (covered.has(k)) continue
		const sep = k.indexOf(':')
		if (sep < 0) continue
		const udk = k.slice(0, sep) as UserDraftItemKind
		const path = k.slice(sep + 1)
		out.push({ key: k, draftKind: udk, deployKind: deployKindOf(udk, udk === 'raw_app'), path })
	}
	return out
}

// ── Pipeline (dot parcours) ─────────────────────────────────────────────────
// Compact progress trail: where the item sits on Draft → Fork → <parent>
// (Draft → <parent> in main context). Derived entirely from the badge state —
// position is the marker (trail fills up to and including `cur`); the badge
// and detail text carry the state in words, so this is never the sole carrier.

export interface Pipeline {
	/** Per-dot tooltip labels: 'Draft', 'Fork', the parent's name. */
	stages: string[]
	/** Index of the current stage — the rightmost filled dot. */
	cur: number
	state: Exclude<BadgeKind, 'none'>
}

/** Undefined for a bare (clean + in-sync, non-terminal) row — nothing to show. */
export function pipelineOf(item: DeployItem, context: SessionContext): Pipeline | undefined {
	const state = badgeOf(item)
	if (state === 'none') return undefined
	const stages = context.isFork
		? ['Draft', context.currentName ?? 'Fork', parentLabel(context)]
		: ['Draft', parentLabel(context)]
	const cur = state === 'draft' ? 0 : state === 'deployed' ? stages.length - 1 : 1
	return { stages, cur, state }
}

// ── Status badge (how the item relates to the parent) ────────────────────────

export type BadgeKind = 'conflict' | 'draft' | 'ahead' | 'deployed' | 'none'

/** The single status badge a row shows, by priority `conflict > draft > ahead`.
 *  A terminal row shows a muted `deployed`; a clean, in-sync row shows nothing.
 *  A pending deletion reads as `ahead` (its action is Delete in parent). */
export function badgeOf(item: DeployItem): BadgeKind {
	if (item.parent === 'conflict') return 'conflict'
	if (item.local) return 'draft'
	if (item.parent === 'ahead') return 'ahead'
	if (item.done) return 'deployed'
	return 'none'
}

/** Per-status tallies over `badgeOf` — the bar's at-a-glance readout, so its
 *  counts always agree with the badges the drawer rows show. */
export function badgeCounts(items: DeployItem[]): Record<BadgeKind, number> {
	const out: Record<BadgeKind, number> = { conflict: 0, draft: 0, ahead: 0, deployed: 0, none: 0 }
	for (const it of items) out[badgeOf(it)]++
	return out
}

// ── Action ──────────────────────────────────────────────────────────────────

export type DeployOp = 'deploy_draft' | 'deploy_item' | 'delete_in_parent' | 'discard' | 'none'

export interface DeployAction {
	op: DeployOp
	label: string
	/** Which pipeline stage this action promotes into. */
	targetStage: 'fork' | 'parent' | null
	secondary?: DeployAction[]
}

function parentLabel(context: SessionContext): string {
	return context.parentName ?? 'parent'
}

/** The axis-driven primary (and secondary) action for a row. On-behalf and
 *  already-deployed gating are layered on at the reactive stage; this returns
 *  the intrinsic next step for the item. */
export function actionFor(item: DeployItem, context: SessionContext): DeployAction {
	const discard: DeployAction = {
		op: 'discard',
		label: item.draftOnly ? 'Discard draft' : 'Discard',
		targetStage: null
	}
	if (item.done) return { op: 'none', label: 'Done', targetStage: null }
	// Conflict: both the fork and the parent changed this item. There is no safe
	// one-click resolution (no merge primitive; auto-pulling the parent would
	// silently discard the session's edits), so the row is informational only —
	// the user reconciles manually in the editor.
	if (item.parent === 'conflict') return { op: 'none', label: 'Conflict', targetStage: null }
	if (item.local) {
		return context.isFork
			? { op: 'deploy_draft', label: 'Deploy to fork', targetStage: 'fork', secondary: [discard] }
			: // Main (non-fork): a draft deploys within its own workspace — no separate
				// parent to name.
				{ op: 'deploy_draft', label: 'Deploy', targetStage: 'parent', secondary: [discard] }
	}
	if (item.parent === 'ahead') {
		return item.removed
			? {
					op: 'delete_in_parent',
					label: `Delete in ${parentLabel(context)}`,
					targetStage: 'parent'
				}
			: { op: 'deploy_item', label: `Deploy to ${parentLabel(context)}`, targetStage: 'parent' }
	}
	// clean + sync (non-terminal, no pending change) — nothing to do.
	return { op: 'none', label: 'Done', targetStage: null }
}

// ── Diff base ───────────────────────────────────────────────────────────────

export type DiffBase =
	| { kind: 'draft'; draftKind: UserDraftItemKind; workspaceId: string; draftOnly: boolean }
	| {
			kind: 'fork_parent'
			deployKind: Kind
			path: string
			forkWorkspaceId: string
			parentWorkspaceId: string
			existsInParent: boolean
			existsInFork: boolean
	  }
	| { kind: 'self'; deployKind: Kind; path: string; workspaceId: string }
	| { kind: 'none' }

/** Where a row's before/after diff values come from. A row with a pending draft
 *  diffs deployed↔draft; a non-draft ahead/conflict row diffs fork↔parent. A
 *  terminal (done) row has nothing to diff but its deployed value is still
 *  loadable (`self`) — the sidebar unwraps a deployed raw app into its files.
 *  The executor maps this to getDraftDiffValues / getItemValue. */
export function diffBaseFor(item: DeployItem, context: SessionContext): DiffBase {
	if (item.local && item.draftKind) {
		return {
			kind: 'draft',
			draftKind: item.draftKind,
			workspaceId: context.currentWorkspaceId,
			draftOnly: item.draftOnly
		}
	}
	if (item.done) {
		return {
			kind: 'self',
			deployKind: item.deployKind,
			path: item.path,
			workspaceId: context.currentWorkspaceId
		}
	}
	if (!context.parentWorkspaceId) return { kind: 'none' }
	return {
		kind: 'fork_parent',
		deployKind: item.deployKind,
		path: item.path,
		forkWorkspaceId: context.currentWorkspaceId,
		parentWorkspaceId: context.parentWorkspaceId,
		existsInParent: item.existsInParent,
		existsInFork: item.existsInFork
	}
}

// ── On-behalf-of eligibility ────────────────────────────────────────────────

/** Kinds that carry a "run on behalf of" identity when promoted fork→parent
 *  (runnables + triggers/schedules). Whether a *specific* item still needs a
 *  choice depends on the source value (async) and is resolved at the reactive
 *  stage; this is the pure kind-level gate. */
export function isOnBehalfEligible(deployKind: Kind): boolean {
	return (
		deployKind === 'flow' ||
		deployKind === 'script' ||
		deployKind === 'app' ||
		deployKind === 'raw_app' ||
		isTriggerOrScheduleKind(deployKind)
	)
}

// ── Deploy plan (executed by the reactive layer) ─────────────────────────────

export interface DeployPlanEntry {
	key: string
	op: Exclude<DeployOp, 'none'>
	deployKind: Kind
	draftKind?: UserDraftItemKind
	path: string
	/** Source workspace (deployItem/update). Undefined for draft deploy/discard,
	 *  which act within `currentWorkspaceId`. */
	workspaceFrom?: string
	/** Target workspace (deployItem/update/delete). */
	workspaceTo?: string
	draftOnly: boolean
	rawApp: boolean
	legacy: boolean
}

/** Concrete util arguments for deploying (or reconciling) one item, derived from
 *  its axes + context. Returns undefined for terminal rows. Mirrors actionFor. */
export function deployPlanFor(
	item: DeployItem,
	context: SessionContext
): DeployPlanEntry | undefined {
	const base = {
		key: item.key,
		deployKind: item.deployKind,
		draftKind: item.draftKind,
		path: item.path,
		draftOnly: item.draftOnly,
		rawApp: item.rawApp,
		legacy: item.legacy
	}
	const parent = context.parentWorkspaceId
	// Terminal rows and conflicts have no automatic action (a conflict has no safe
	// one-click resolution — it's reconciled manually).
	if (item.done || item.parent === 'conflict') return undefined
	if (item.local) {
		// deployDraft acts within the current workspace (draft → deployed there).
		return { ...base, op: 'deploy_draft' }
	}
	if (item.parent === 'ahead') {
		if (!parent) return undefined
		if (item.removed) return { ...base, op: 'delete_in_parent', workspaceTo: parent }
		return {
			...base,
			op: 'deploy_item',
			workspaceFrom: context.currentWorkspaceId,
			workspaceTo: parent
		}
	}
	return undefined
}

/** Discard plan for a draft row (secondary action). */
export function discardPlanFor(
	item: DeployItem,
	context: SessionContext
): DeployPlanEntry | undefined {
	if (!item.local || !item.draftKind) return undefined
	return {
		key: item.key,
		op: 'discard',
		deployKind: item.deployKind,
		draftKind: item.draftKind,
		path: item.path,
		workspaceTo: context.currentWorkspaceId,
		draftOnly: item.draftOnly,
		rawApp: item.rawApp,
		legacy: item.legacy
	}
}
