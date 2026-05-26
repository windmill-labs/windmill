import { deepEqual } from 'fast-equals'
import { UserDraft, type UserDraftItemKind } from '$lib/userDraft.svelte'
import { getItemValue } from '$lib/utils_workspace_deploy'
import type { Kind } from '$lib/utils_deployable'
import type { WorkspaceComparison, WorkspaceItemDiff } from '$lib/gen'

// The backend `compareWorkspaces` API diffs the fork's *committed* (deployed)
// state against its parent. It cannot see local drafts, which live only in
// the browser's localStorage (the `UserDraft` store, keyed by the fork
// workspace id). This module augments a `WorkspaceComparison` with those
// local drafts so the Fork Diff Viewer / compare page can surface
// uncommitted session changes for review.
//
// Local-draft items are flagged `localDraft: true`. They are NOT in the
// fork's backend DB, so they must be shown read-only and excluded from the
// compare page's deployable set.

export type ForkDiffKind = WorkspaceItemDiff['kind']

export type AugmentedWorkspaceItemDiff = WorkspaceItemDiff & {
	/** Case 1: the item exists on the fork server (deployed) AND its local
	 * (localStorage) draft differs from that server value. Deploying drops the
	 * local changes — rendered with a warning + "show local changes" diff. */
	localChanges?: boolean
	/** Case 2: the item exists ONLY as a local draft (not on the fork server).
	 * It cannot be deployed until saved in the fork — rendered dimmed. */
	newLocalDraft?: boolean
	/** Row was synthesized from a local draft (no corresponding entry in the
	 * backend fork-vs-parent diff) → not deployable from the compare page. */
	localOnly?: boolean
}
export type AugmentedWorkspaceComparison = Omit<WorkspaceComparison, 'diffs'> & {
	diffs: AugmentedWorkspaceItemDiff[]
}

// UserDraft kind → compare-API kind. Kinds without a `WorkspaceItemDiff`
// equivalent (trigger_poll / cli / nextcloud / google / github) are omitted
// and skipped during augmentation.
const DRAFT_KIND_TO_FORK_KIND: Partial<Record<UserDraftItemKind, ForkDiffKind>> = {
	script: 'script',
	flow: 'flow',
	app: 'app',
	raw_app: 'raw_app',
	resource: 'resource',
	variable: 'variable',
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
	trigger_email: 'email_trigger',
	trigger_default_email: 'email_trigger'
}

// Reverse map (first draft kind wins for shared targets like email_trigger).
const FORK_KIND_TO_DRAFT_KIND: Partial<Record<ForkDiffKind, UserDraftItemKind>> = (() => {
	const out: Partial<Record<ForkDiffKind, UserDraftItemKind>> = {}
	for (const [draftKind, forkKind] of Object.entries(DRAFT_KIND_TO_FORK_KIND)) {
		if (forkKind && !(forkKind in out)) out[forkKind] = draftKind as UserDraftItemKind
	}
	return out
})()

function diffKey(kind: string, path: string): string {
	return `${kind}/${path}`
}

// Project a draft value and a deployed item value to the same comparable
// shape so a draft that merely mirrors the deployed item (e.g. from opening
// an item in the session preview without editing) is recognised as
// "no change". Best-effort for raw_app (deployed apps nest content under
// `value`, drafts keep it flat).
function comparableProjection(kind: ForkDiffKind, v: any): unknown {
	if (v == null) return v
	if (kind === 'script') {
		return { content: v.content, language: v.language, summary: v.summary, schema: v.schema }
	}
	if (kind === 'flow') {
		return { value: v.value, schema: v.schema, summary: v.summary }
	}
	if (kind === 'raw_app') {
		const value = v.value ?? v
		return {
			files: value?.files ?? v.files,
			runnables: value?.runnables ?? v.runnables,
			summary: v.summary
		}
	}
	return v
}

// JSON round-trip both sides (drops `undefined`-valued keys, normalizes) then
// deep-compare — mirrors `normalizeForCompare` in userDraft.svelte.ts.
function normalize(v: unknown): unknown {
	if (v === undefined) return undefined
	try {
		return JSON.parse(JSON.stringify(v))
	} catch {
		return v
	}
}

// `getItemValue` resolves to an empty object `{}` (rather than throwing or
// returning null) when the item doesn't exist on the server. Treat that — and
// null/undefined — as "absent".
function isPresent(v: unknown): boolean {
	if (v == null) return false
	if (typeof v === 'object') return Object.keys(v as object).length > 0
	return true
}

function draftDiffersFromDeployed(
	kind: ForkDiffKind,
	draftValue: unknown,
	deployedValue: unknown
): boolean {
	return !deepEqual(
		normalize(comparableProjection(kind, draftValue)),
		normalize(comparableProjection(kind, deployedValue))
	)
}

/**
 * Merge local drafts (browser localStorage, scoped to `forkWorkspaceId`) into
 * a backend `WorkspaceComparison`:
 *  - a draft matching an existing diff flags that diff `localDraft` (its fork
 *    side should be read from the draft);
 *  - a draft with no matching diff is added as a synthetic `localDraft` entry,
 *    after filtering no-op baseline drafts (draft identical to the deployed
 *    item) for the loader-seeded kinds.
 *
 * Async: it fetches the fork's deployed value for draft-only candidates to
 * decide whether the draft is a real change. The number of such fetches is
 * bounded by how many items the session touched.
 */
export async function augmentForkComparisonWithLocalDrafts(
	comparison: WorkspaceComparison,
	forkWorkspaceId: string
): Promise<AugmentedWorkspaceComparison> {
	const diffs: AugmentedWorkspaceItemDiff[] = comparison.diffs.map((d) => ({ ...d }))
	const byKey = new Map<string, AugmentedWorkspaceItemDiff>()
	for (const d of diffs) byKey.set(diffKey(d.kind, d.path), d)

	const drafts = UserDraft.list({ workspace: forkWorkspaceId })

	for (const entry of drafts) {
		const forkKind = DRAFT_KIND_TO_FORK_KIND[entry.itemKind]
		if (!forkKind) continue
		// Skip "new item" scaffold drafts stored at an empty path — they aren't
		// real workspace items yet and would otherwise render as a pathless,
		// summary-less "local draft" row (duplicating the real, named entry).
		if (!entry.path || !entry.path.trim()) continue
		const key = diffKey(forkKind, entry.path)

		// The fork's server (deployed) value, used to (a) tell a real local edit
		// from a no-op baseline draft (the session loaders seed a draft equal to
		// the loaded value on open) and (b) decide whether the item is on the
		// fork server at all (Case 1 vs Case 2).
		let serverValue: unknown
		try {
			serverValue = await getItemValue(forkKind as Kind, entry.path, forkWorkspaceId)
		} catch {
			serverValue = undefined
		}
		const onServer = isPresent(serverValue)
		const differs = !onServer || draftDiffersFromDeployed(forkKind, entry.value, serverValue)

		// Draft equals the server value → no local change. Leave any existing
		// backend diff untouched and add no synthetic row.
		if (onServer && !differs) continue

		const existing = byKey.get(key)
		if (existing) {
			// Case 1: a backend fork-vs-parent diff that also carries a divergent
			// local draft. Stays deployable (deploys the server value); the local
			// changes would be dropped — flagged for a warning.
			existing.localChanges = true
			continue
		}

		if (onServer) {
			// Case 1 with no fork-vs-parent delta (server == parent): review-only.
			const synthetic: AugmentedWorkspaceItemDiff = {
				kind: forkKind,
				path: entry.path,
				ahead: 1,
				behind: 0,
				has_changes: true,
				exists_in_source: true,
				exists_in_fork: true,
				localChanges: true,
				localOnly: true
			}
			diffs.push(synthetic)
			byKey.set(key, synthetic)
		} else {
			// Case 2: brand-new local item, not on the fork server → cannot deploy.
			const synthetic: AugmentedWorkspaceItemDiff = {
				kind: forkKind,
				path: entry.path,
				ahead: 1,
				behind: 0,
				has_changes: true,
				exists_in_source: false,
				exists_in_fork: true,
				newLocalDraft: true,
				localOnly: true
			}
			diffs.push(synthetic)
			byKey.set(key, synthetic)
		}
	}

	const added = diffs.length - comparison.diffs.length
	const summary = {
		...comparison.summary,
		total_diffs: comparison.summary.total_diffs + added,
		total_ahead: comparison.summary.total_ahead + added
	}

	return { ...comparison, diffs, summary }
}

/**
 * Fork-side value for a diff item: the local draft when one exists, else the
 * deployed value from the backend. Use this (instead of `getItemValue` with
 * the fork workspace) so the per-item diff shows pending local-draft content.
 */
export async function getForkItemValue(
	kind: Kind,
	path: string,
	forkWorkspaceId: string
): Promise<unknown> {
	const draftKind = FORK_KIND_TO_DRAFT_KIND[kind as ForkDiffKind]
	if (draftKind) {
		const draft = UserDraft.get(draftKind, path, { workspace: forkWorkspaceId })
		if (draft != null) return draft
	}
	return getItemValue(kind, path, forkWorkspaceId)
}
