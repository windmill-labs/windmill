import type { UserDraftItemKind } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import type { Kind } from '$lib/utils_deployable'
import { maskKey } from './modifiedItemsMask'

// Unified item model for the session Review & Deploy surface. This is the pure,
// UI-free core: every item the session touched is either a pending **draft** or
// **deployed** in the session workspace (terminal). Deploying always lands in
// the session's own workspace — promoting a fork's changes to its parent is the
// compare page's concern, not modeled here.
//
// Kept free of Svelte runes and async so it unit-tests in the node project. The
// reactive wrapper (data loading, deploy execution) lives in the sibling
// `.svelte.ts` and consumes these functions.

// ── Item taxonomy bridge ────────────────────────────────────────────────────
// The draft list speaks UserDraftItemKind (`trigger_schedule`, `raw_app`); the
// deploy utils speak the layout `Kind` (`schedule`, `raw_app`, `http_trigger`).
// This maps draft→deploy to key deploy calls and row icons. Only non-identity
// cases listed; everything else is identity.
const DEPLOY_KIND_BY_DRAFT_KIND: Partial<Record<UserDraftItemKind, Kind>> = {
	trigger_schedule: 'schedule',
	trigger_http: 'http_trigger',
	trigger_websocket: 'websocket_trigger',
	trigger_kafka: 'kafka_trigger',
	trigger_nats: 'nats_trigger',
	trigger_postgres: 'postgres_trigger',
	trigger_mqtt: 'mqtt_trigger',
	trigger_amqp: 'amqp_trigger',
	trigger_sqs: 'sqs_trigger',
	trigger_gcp: 'gcp_trigger',
	trigger_azure: 'azure_trigger',
	// Both email draft kinds deploy as the one email trigger kind: the editor
	// saves per-path email-trigger drafts as `trigger_email`, the fork diff
	// speaks `trigger_default_email`.
	trigger_default_email: 'email_trigger',
	trigger_email: 'email_trigger',
	raw_app: 'raw_app'
}

function deployKindOf(draftKind: UserDraftItemKind, rawApp: boolean): Kind {
	if (rawApp) return 'raw_app'
	return DEPLOY_KIND_BY_DRAFT_KIND[draftKind] ?? (draftKind as unknown as Kind)
}

// ── Public types ────────────────────────────────────────────────────────────

export interface DeployItem {
	/** Canonical identity `${UserDraftItemKind}:${path}`. */
	key: string
	/** Layout/deploy kind (script, flow, raw_app, http_trigger, schedule…). */
	deployKind: Kind
	/** UserDraftItemKind for draft ops. */
	draftKind: UserDraftItemKind
	path: string
	/** Friendly path for display; storage `path` stays the op/edit key. */
	displayPath: string
	summary?: string
	/** Deployed with no pending draft — the terminal state. */
	done: boolean
	hasDraft: boolean
	draftOnly: boolean
	draftUsers?: { username?: string | null }[]
	/** Own draft (deployable/discardable). */
	mine: boolean
	canWrite: boolean
	/** Legacy workspace-level draft (drives discard's `legacy` flag). */
	legacy: boolean
	rawApp: boolean
}

export interface BuildInput {
	draftItems: DraftItem[]
	/** Chat-modified-items mask (`UserDraftItemKind:path`). When set, only masked
	 *  items are included, and mask-only entries (no draft) surface as terminal
	 *  deployed rows — but only when confirmed to still exist (see `existingKeys`). */
	mask?: Set<string>
	/** Mask keys confirmed to still exist in the workspace (deployed). A mask-only
	 *  key that isn't here is treated as gone (a discarded draft or a deleted item)
	 *  and dropped, so it doesn't masquerade as deployed. Undefined → no mask-only
	 *  rows are emitted yet (existence still resolving). */
	existingKeys?: Set<string>
}

// ── Build: draft list + mask-only deployed rows into unified items ──────────

export function buildDeployItems(input: BuildInput): DeployItem[] {
	const { draftItems, mask, existingKeys } = input
	const scopedDrafts = mask
		? draftItems.filter((it) => mask.has(maskKey(it.kind, it.path)))
		: draftItems

	const out: DeployItem[] = []
	const seen = new Set<string>()

	// 1. Pending drafts.
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
			done: false,
			hasDraft: true,
			draftOnly: it.draft_only,
			draftUsers: it.draft_users,
			mine: it.mine,
			canWrite: it.can_write,
			legacy: it.legacy_draft,
			rawApp: it.raw_app
		})
	}

	// 2. Mask-only items (touched by the chat, no pending draft) → terminal
	//    deployed rows, but ONLY when confirmed to still exist. A discarded draft
	//    or a deleted item leaves a mask entry with no item; without this guard it
	//    would masquerade as deployed.
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
				done: true,
				hasDraft: false,
				draftOnly: false,
				draftUsers: undefined,
				mine: true,
				canWrite: true,
				legacy: false,
				rawApp: udk === 'raw_app'
			})
		}
	}

	return out
}

/** Mask keys not covered by a current draft — the candidates for a terminal
 *  deployed row. The reactive layer existence-checks these (in the session
 *  workspace) to tell a deployed item from a discarded/deleted one, then feeds
 *  the survivors back as `existingKeys`. */
export function maskOnlyCandidates(
	input: Omit<BuildInput, 'existingKeys'>
): { key: string; deployKind: Kind; draftKind: UserDraftItemKind; path: string }[] {
	const { draftItems, mask } = input
	if (!mask) return []
	const covered = new Set<string>()
	for (const it of draftItems) {
		const k = maskKey(it.kind, it.path)
		if (mask.has(k)) covered.add(k)
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

// ── Status badge ─────────────────────────────────────────────────────────────

export type BadgeKind = 'draft' | 'deployed'

/** The single status badge a row shows: a pending draft, or deployed (green,
 *  terminal). Every built row is one of the two. */
export function badgeOf(item: DeployItem): BadgeKind {
	return item.done ? 'deployed' : 'draft'
}

/** Per-status tallies over `badgeOf` — the bar's at-a-glance readout, so its
 *  counts always agree with the badges the drawer rows show. */
export function badgeCounts(items: DeployItem[]): Record<BadgeKind, number> {
	const out: Record<BadgeKind, number> = { draft: 0, deployed: 0 }
	for (const it of items) out[badgeOf(it)]++
	return out
}

// ── Action ──────────────────────────────────────────────────────────────────

export type DeployOp = 'deploy_draft' | 'discard' | 'none'

export interface DeployAction {
	op: DeployOp
	label: string
	secondary?: DeployAction[]
}

/** The primary (and secondary) action for a row: a draft deploys into the
 *  session workspace, a deployed row has nothing left to do. */
export function actionFor(item: DeployItem): DeployAction {
	if (item.done) return { op: 'none', label: 'Done' }
	return {
		op: 'deploy_draft',
		label: 'Deploy',
		secondary: [{ op: 'discard', label: item.draftOnly ? 'Discard draft' : 'Discard' }]
	}
}

// ── Diff base ───────────────────────────────────────────────────────────────

export type DiffBase =
	| { kind: 'draft'; draftKind: UserDraftItemKind; workspaceId: string; draftOnly: boolean }
	| { kind: 'self'; deployKind: Kind; path: string; workspaceId: string }

/** Where a row's before/after diff values come from. A draft row diffs
 *  deployed↔draft; a deployed row has nothing to diff but its value is still
 *  loadable (`self`) — the sidebar unwraps a deployed raw app into its files.
 *  The executor maps this to getDraftDiffValues / getItemValue. */
export function diffBaseFor(item: DeployItem, workspaceId: string): DiffBase {
	if (item.hasDraft) {
		return { kind: 'draft', draftKind: item.draftKind, workspaceId, draftOnly: item.draftOnly }
	}
	return { kind: 'self', deployKind: item.deployKind, path: item.path, workspaceId }
}

// ── Deploy plan (executed by the reactive layer) ─────────────────────────────

export interface DeployPlanEntry {
	key: string
	op: Exclude<DeployOp, 'none'>
	draftKind: UserDraftItemKind
	path: string
	draftOnly: boolean
	rawApp: boolean
	legacy: boolean
}

/** Concrete util arguments for deploying one item's draft into the session
 *  workspace. Returns undefined for deployed (terminal) rows. Mirrors actionFor. */
export function deployPlanFor(item: DeployItem): DeployPlanEntry | undefined {
	if (!item.hasDraft) return undefined
	return {
		key: item.key,
		op: 'deploy_draft',
		draftKind: item.draftKind,
		path: item.path,
		draftOnly: item.draftOnly,
		rawApp: item.rawApp,
		legacy: item.legacy
	}
}

/** Discard plan for a draft row (secondary action). */
export function discardPlanFor(item: DeployItem): DeployPlanEntry | undefined {
	if (!item.hasDraft) return undefined
	return {
		key: item.key,
		op: 'discard',
		draftKind: item.draftKind,
		path: item.path,
		draftOnly: item.draftOnly,
		rawApp: item.rawApp,
		legacy: item.legacy
	}
}
