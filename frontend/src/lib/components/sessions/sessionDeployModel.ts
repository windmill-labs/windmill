import type { UserDraftItemKind, WorkspaceComparison } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import type { Kind } from '$lib/utils_deployable'
import { isTriggerOrScheduleKind } from 'windmill-utils-internal'
import { maskKey, forkDiffKindToUserDraftKind } from './modifiedItemsMask'

// Unified item model for the session Review & Deploy surface. This is the pure,
// UI-free core: it merges the two data sources the compare page keeps apart —
// the per-user draft list (Deployed ↔ draft) and the fork comparison
// (fork ↔ parent) — into ONE list of items, each carrying a resolved lifecycle
// `state` plus pure derivations (pipeline, action, diff base, selection,
// segments, deploy plan) the drawer renders and executes.
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

/** Lifecycle position of an item within the session's deploy pipeline.
 *  `draft` takes precedence over `conflict`/`in_fork` when a pending draft
 *  exists — the unsaved edit is the first thing to promote. */
export type DeployItemState = 'draft' | 'in_fork' | 'in_parent' | 'deleted' | 'conflict'

/** Left-pane filter segments. `to_review` = everything not terminal; `all` =
 *  every item; the rest are the exclusive lifecycle buckets. */
export type DeploySegment = 'to_review' | 'drafts' | 'in_fork' | 'done' | 'all'

export interface SessionContext {
	/** Fork (Draft→Fork→Parent) vs main (Draft→Parent, no fork stage). */
	isFork: boolean
	currentWorkspaceId: string
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
	state: DeployItemState
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

function stateFrom(opts: {
	hasDraft: boolean
	ahead: number
	behind: number
	existsInFork: boolean
	existsInParent: boolean
}): DeployItemState {
	const { hasDraft, ahead, behind, existsInFork, existsInParent } = opts
	// Pending edit first — you promote the draft to the fork before reconciling
	// fork↔parent.
	if (hasDraft) return 'draft'
	if (ahead > 0 && behind > 0) return 'conflict'
	if (!existsInFork && existsInParent) return 'deleted'
	if (ahead > 0 || behind > 0) return 'in_fork'
	return 'in_parent'
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
				state: stateFrom({
					hasDraft: !!draft,
					ahead: d.ahead,
					behind: d.behind,
					existsInFork,
					existsInParent
				}),
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
			state: 'draft',
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
				state: 'in_parent',
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

// ── Pipeline ────────────────────────────────────────────────────────────────

export type PipelineStatus = 'done' | 'current' | 'todo' | 'blocked'
export interface PipelineStage {
	id: 'draft' | 'fork' | 'parent'
	status: PipelineStatus
}

/** The 3-dot (fork) / 2-dot (main) pipeline indicator for an item. */
export function pipelineOf(item: DeployItem, context: SessionContext): PipelineStage[] {
	const parent: PipelineStatus = item.state === 'in_parent' ? 'done' : 'todo'
	if (!context.isFork) {
		// Main: Draft → Parent, no fork stage.
		const draft: PipelineStatus = item.state === 'draft' ? 'current' : 'done'
		return [
			{ id: 'draft', status: draft },
			{ id: 'parent', status: parent }
		]
	}
	let draft: PipelineStatus
	let fork: PipelineStatus
	switch (item.state) {
		case 'draft':
			draft = 'current'
			fork = 'todo'
			break
		case 'conflict':
			draft = 'done'
			fork = 'blocked'
			break
		case 'in_parent':
			draft = 'done'
			fork = 'done'
			break
		default: // in_fork, deleted
			draft = 'done'
			fork = 'current'
	}
	return [
		{ id: 'draft', status: draft },
		{ id: 'fork', status: fork },
		{ id: 'parent', status: parent }
	]
}

// ── Action ──────────────────────────────────────────────────────────────────

export type DeployOp =
	| 'deploy_draft'
	| 'deploy_item'
	| 'delete_in_parent'
	| 'update_fork'
	| 'discard'
	| 'none'

export interface DeployAction {
	op: DeployOp
	label: string
	/** Which pipeline stage this action promotes into (drives footer grouping). */
	targetStage: 'fork' | 'parent' | null
	/** Present → render disabled with this reason (conflict, done). */
	blockedReason?: string
	secondary?: DeployAction[]
}

function parentLabel(context: SessionContext): string {
	return context.parentName ?? 'parent'
}

/** The state-driven primary (and secondary) action for a row. On-behalf and
 *  already-deployed gating are layered on at the reactive stage; this returns
 *  the intrinsic action for the item's state. */
export function actionFor(item: DeployItem, context: SessionContext): DeployAction {
	const discard: DeployAction = {
		op: 'discard',
		label: item.draftOnly ? 'Discard draft' : 'Discard',
		targetStage: null
	}
	switch (item.state) {
		case 'draft':
			return context.isFork
				? { op: 'deploy_draft', label: 'Deploy to fork', targetStage: 'fork', secondary: [discard] }
				: // Main (non-fork): a draft deploys within its own workspace — there is
					// no separate parent to name.
					{ op: 'deploy_draft', label: 'Deploy', targetStage: 'parent', secondary: [discard] }
		case 'in_fork':
			// ahead → promote to parent; behind-only → the parent moved, update fork.
			if (item.ahead > 0) {
				return {
					op: 'deploy_item',
					label: `Deploy to ${parentLabel(context)}`,
					targetStage: 'parent'
				}
			}
			return { op: 'update_fork', label: 'Update fork', targetStage: 'fork' }
		case 'deleted':
			return {
				op: 'delete_in_parent',
				label: `Delete in ${parentLabel(context)}`,
				targetStage: 'parent'
			}
		case 'conflict':
			return {
				op: 'none',
				label: `Deploy to ${parentLabel(context)}`,
				targetStage: 'parent',
				blockedReason: 'Resolve the conflict before deploying'
			}
		case 'in_parent':
			return { op: 'none', label: 'Done', targetStage: null }
	}
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
	| { kind: 'none' }

/** Where a row's before/after diff values come from, given its state. A draft
 *  row diffs deployed↔draft; an in-fork/conflict/deleted row diffs fork↔parent.
 *  The executor maps this to getDraftDiffValues / getItemValue×2. */
export function diffBaseFor(item: DeployItem, context: SessionContext): DiffBase {
	if (item.state === 'draft' && item.draftKind) {
		return {
			kind: 'draft',
			draftKind: item.draftKind,
			workspaceId: context.currentWorkspaceId,
			draftOnly: item.draftOnly
		}
	}
	if (item.state === 'in_parent') return { kind: 'none' }
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

// ── Selection ───────────────────────────────────────────────────────────────

/** A row is selectable when it has an actionable deploy in its current state and
 *  — for draft-stage ops — the draft is the user's own with write permission.
 *  Conflicts are excluded (blocked until resolved). Already-deployed exclusion is
 *  applied by the reactive layer (it owns per-row deploymentStatus). */
export function selectableOf(item: DeployItem): boolean {
	// Actionable states only. Terminal (in_parent) and blocked (conflict) rows are
	// never selectable regardless of context.
	if (item.state === 'in_parent' || item.state === 'conflict') return false
	if (item.state === 'draft') return item.mine && item.canWrite
	return true // in_fork, deleted
}

/** Handoff default: every selectable row checked. */
export function defaultSelection(items: DeployItem[]): string[] {
	return items.filter(selectableOf).map((i) => i.key)
}

// ── Segments ────────────────────────────────────────────────────────────────

export function inSegment(item: DeployItem, seg: DeploySegment): boolean {
	switch (seg) {
		case 'all':
			return true
		case 'to_review':
			return item.state !== 'in_parent'
		case 'drafts':
			return item.state === 'draft'
		case 'in_fork':
			return item.state === 'in_fork'
		case 'done':
			return item.state === 'in_parent'
	}
}

export function itemsInSegment(items: DeployItem[], seg: DeploySegment): DeployItem[] {
	return items.filter((i) => inSegment(i, seg))
}

export function segmentCounts(items: DeployItem[]): Record<DeploySegment, number> {
	const counts: Record<DeploySegment, number> = {
		to_review: 0,
		drafts: 0,
		in_fork: 0,
		done: 0,
		all: 0
	}
	for (const item of items) {
		for (const seg of ['to_review', 'drafts', 'in_fork', 'done', 'all'] as DeploySegment[]) {
			if (inSegment(item, seg)) counts[seg]++
		}
	}
	return counts
}

// ── Footer summary ──────────────────────────────────────────────────────────

export interface FooterSummary {
	/** Selected rows deploying to the fork stage (drafts, fork context). */
	toFork: number
	/** Selected rows deploying to the parent (in-fork promotions, deletions, and
	 *  drafts in main context). */
	toParent: number
	/** Selected on-behalf-eligible parent promotions (upper bound on "needs
	 *  on-behalf-of"; the reactive layer subtracts resolved choices). */
	onBehalfEligible: number
	/** Selected conflicts (blocked). */
	conflicts: number
}

export function footerSummary(
	items: DeployItem[],
	selectedKeys: Set<string>,
	context: SessionContext
): FooterSummary {
	const summary: FooterSummary = { toFork: 0, toParent: 0, onBehalfEligible: 0, conflicts: 0 }
	for (const item of items) {
		if (!selectedKeys.has(item.key)) continue
		if (item.state === 'conflict') summary.conflicts++
		const action = actionFor(item, context)
		if (action.targetStage === 'fork') summary.toFork++
		else if (action.targetStage === 'parent' && action.op !== 'none') {
			summary.toParent++
			if (action.op === 'deploy_item' && isOnBehalfEligible(item.deployKind)) {
				summary.onBehalfEligible++
			}
		}
	}
	return summary
}

// ── Deploy plan (executed by the reactive layer / S3) ───────────────────────

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

/** Concrete util arguments for deploying (or discarding) one item, derived from
 *  its state + context. Returns undefined for terminal/blocked rows. */
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
	switch (item.state) {
		case 'draft':
			// deployDraft acts within the current workspace (draft → deployed there).
			return { ...base, op: 'deploy_draft' }
		case 'in_fork':
			if (item.ahead > 0) {
				if (!parent) return undefined
				return {
					...base,
					op: 'deploy_item',
					workspaceFrom: context.currentWorkspaceId,
					workspaceTo: parent
				}
			}
			if (!parent) return undefined
			return {
				...base,
				op: 'update_fork',
				workspaceFrom: parent,
				workspaceTo: context.currentWorkspaceId
			}
		case 'deleted':
			if (!parent) return undefined
			return { ...base, op: 'delete_in_parent', workspaceTo: parent }
		case 'conflict':
		case 'in_parent':
			return undefined
	}
}

/** Discard plan for a draft row (secondary action). */
export function discardPlanFor(
	item: DeployItem,
	context: SessionContext
): DeployPlanEntry | undefined {
	if (item.state !== 'draft' || !item.draftKind) return undefined
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
