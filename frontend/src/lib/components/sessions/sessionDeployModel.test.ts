import { describe, it, expect } from 'vitest'
import type { UserDraftItemKind, WorkspaceComparison, WorkspaceItemDiff } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	buildDeployItems,
	maskOnlyCandidates,
	badgeOf,
	pipelineOf,
	actionFor,
	diffBaseFor,
	isOnBehalfEligible,
	badgeCounts,
	deployPlanFor,
	discardPlanFor,
	type DeployItem,
	type SessionContext
} from './sessionDeployModel'
import { maskKey } from './modifiedItemsMask'

// ── Fixtures ────────────────────────────────────────────────────────────────

function draft(kind: UserDraftItemKind, path: string, over: Partial<DraftItem> = {}): DraftItem {
	return {
		kind,
		path,
		summary: undefined,
		draft_path: undefined,
		draft_only: false,
		legacy_draft: false,
		raw_app: kind === 'raw_app',
		can_write: true,
		draft_users: undefined,
		mine: true,
		...over
	}
}

function diff(
	kind: WorkspaceItemDiff['kind'],
	path: string,
	over: Partial<WorkspaceItemDiff> = {}
): WorkspaceItemDiff {
	return {
		kind,
		path,
		ahead: 1,
		behind: 0,
		has_changes: true,
		exists_in_source: true,
		exists_in_fork: true,
		...over
	}
}

function comparison(diffs: WorkspaceItemDiff[]): WorkspaceComparison {
	return { diffs } as WorkspaceComparison
}

const forkCtx: SessionContext = {
	isFork: true,
	currentWorkspaceId: 'wm-fork-f',
	parentWorkspaceId: 'parent',
	parentName: 'test-ai-sessions'
}
const mainCtx: SessionContext = { isFork: false, currentWorkspaceId: 'ws' }

function byKey(items: DeployItem[]): Map<string, DeployItem> {
	return new Map(items.map((i) => [i.key, i]))
}

// ── buildDeployItems: merge + axis derivation ───────────────────────────────

describe('buildDeployItems — axis derivation', () => {
	it('a draft-only item is local, parent=sync, not done, exists nowhere yet', () => {
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/foo', { draft_only: true })],
			context: mainCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].local).toBe(true)
		expect(items[0].parent).toBe('sync')
		expect(items[0].done).toBe(false)
		expect(items[0].draftOnly).toBe(true)
		expect(items[0].existsInParent).toBe(false)
	})

	it('a fork diff ahead-only (no draft) is parent=ahead, not local', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/foo', { ahead: 2, behind: 0 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].parent).toBe('ahead')
		expect(items[0].local).toBe(false)
		expect(items[0].ahead).toBe(2)
	})

	it('ahead AND behind is parent=conflict', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/bar', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].parent).toBe('conflict')
	})

	it('behind-only fork diff (parent moved, fork did not) falls to parent=sync', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/beh', { ahead: 0, behind: 2 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].parent).toBe('sync')
	})

	it('removed in fork (present in parent) is removed=true, parent=ahead', () => {
		const items = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/gone', { ahead: 1, exists_in_fork: false, exists_in_source: true })
			]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].removed).toBe(true)
		expect(items[0].parent).toBe('ahead')
	})

	it('a pending draft on a deployed fork item keeps both axes, merged to ONE row', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/foo', { ahead: 1, behind: 0 })]),
			draftItems: [draft('script', 'u/a/foo')],
			context: forkCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].local).toBe(true)
		expect(items[0].parent).toBe('ahead')
		expect(items[0].key).toBe(maskKey('script', 'u/a/foo'))
	})

	it('unifies the trigger taxonomy (schedule diff ↔ trigger_schedule draft) into one row', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('schedule', 'u/a/sched')]),
			draftItems: [draft('trigger_schedule', 'u/a/sched')],
			context: forkCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].deployKind).toBe('schedule')
		expect(items[0].draftKind).toBe('trigger_schedule')
		expect(items[0].local).toBe(true)
	})
})

describe('buildDeployItems — scoping and Done', () => {
	it('mask scopes both drafts and diffs to touched items', () => {
		const mask = new Set([maskKey('script', 'u/a/keep')])
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/keep'), diff('script', 'u/a/drop')]),
			draftItems: [draft('flow', 'u/a/other')],
			mask,
			context: forkCtx
		})
		expect(items.map((i) => i.path)).toEqual(['u/a/keep'])
	})

	it('mask-only entries that still exist surface as terminal done rows', () => {
		const key = maskKey('script', 'u/a/done')
		const items = buildDeployItems({
			comparison: comparison([]),
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].done).toBe(true)
		expect(items[0].parent).toBe('sync')
		expect(items[0].path).toBe('u/a/done')
	})

	it('a mask-only entry that no longer exists (discarded draft) is dropped', () => {
		const key = maskKey('script', 'u/a/gone')
		const pending = buildDeployItems({
			comparison: comparison([]),
			draftItems: [],
			mask: new Set([key]),
			context: forkCtx
		})
		expect(pending).toHaveLength(0)
		const resolved = buildDeployItems({
			comparison: comparison([]),
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set(),
			context: forkCtx
		})
		expect(resolved).toHaveLength(0)
	})

	it('maskOnlyCandidates returns keys not covered by a draft or diff', () => {
		const cands = maskOnlyCandidates({
			comparison: comparison([diff('script', 'u/a/infork', { ahead: 1 })]),
			draftItems: [draft('script', 'u/a/draft')],
			mask: new Set([
				maskKey('script', 'u/a/infork'),
				maskKey('script', 'u/a/draft'),
				maskKey('trigger_schedule', 'u/a/done')
			]),
			context: forkCtx
		})
		expect(cands.map((c) => c.key)).toEqual([maskKey('trigger_schedule', 'u/a/done')])
		expect(cands[0].deployKind).toBe('schedule')
	})

	it('main (non-fork) context ignores comparison and lists drafts only', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/forkonly')]),
			draftItems: [draft('script', 'u/a/d', { draft_only: true })],
			context: mainCtx
		})
		expect(items.map((i) => i.path)).toEqual(['u/a/d'])
		expect(items[0].local).toBe(true)
	})

	it('resource_type / folder diffs (no UserDraftItemKind) are skipped', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('folder', 'f/x'), diff('resource_type', 'rt')]),
			draftItems: [],
			context: forkCtx
		})
		expect(items).toHaveLength(0)
	})
})

// ── badgeOf: single badge by priority ────────────────────────────────────────

describe('badgeOf', () => {
	function only(diffs: WorkspaceItemDiff[], drafts: DraftItem[] = []): DeployItem {
		return buildDeployItems({
			comparison: comparison(diffs),
			draftItems: drafts,
			context: forkCtx
		})[0]
	}

	it('draft-only → draft', () => {
		expect(badgeOf(only([], [draft('script', 'u/a/d')]))).toBe('draft')
	})
	it('ahead (no draft) → ahead', () => {
		expect(badgeOf(only([diff('script', 'u/a/a', { ahead: 1 })]))).toBe('ahead')
	})
	it('behind-only (parent moved, fork untouched) is not a badge state → none', () => {
		expect(badgeOf(only([diff('script', 'u/a/b', { ahead: 0, behind: 2 })]))).toBe('none')
	})
	it('conflict wins over a pending draft', () => {
		const it = only([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })], [draft('flow', 'u/a/c')])
		expect(it.local).toBe(true)
		expect(badgeOf(it)).toBe('conflict')
	})
	it('draft wins over ahead (losing axis hidden from the badge)', () => {
		const it = only([diff('script', 'u/a/da', { ahead: 1 })], [draft('script', 'u/a/da')])
		expect(it.parent).toBe('ahead')
		expect(badgeOf(it)).toBe('draft')
	})
	it('done → deployed', () => {
		const key = maskKey('script', 'u/a/done')
		const [it] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(badgeOf(it)).toBe('deployed')
	})
	it('a clean in-sync row → none; a done row → deployed (op, not status, carries delete)', () => {
		const cleanSync = { local: false, parent: 'sync', done: false, removed: false } as DeployItem
		expect(badgeOf(cleanSync)).toBe('none')
		const done = { local: false, parent: 'sync', done: true, removed: false } as DeployItem
		expect(badgeOf(done)).toBe('deployed')
	})
})

// ── pipeline ────────────────────────────────────────────────────────────────

describe('pipelineOf', () => {
	it('fork draft: current = Draft (stage 0), stages carry the parent name', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: forkCtx })
		const p = pipelineOf(item, forkCtx)
		expect(p).toEqual({ stages: ['Draft', 'Fork', 'test-ai-sessions'], cur: 0, state: 'draft' })
	})

	it("middle stage carries the fork's name when the context has one", () => {
		const named = { ...forkCtx, currentName: 'my-fork' }
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: named })
		expect(pipelineOf(item, named)?.stages).toEqual(['Draft', 'my-fork', 'test-ai-sessions'])
	})

	it('ahead (in fork): current = Fork (stage 1)', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f')]),
			draftItems: [],
			context: forkCtx
		})
		const p = pipelineOf(item, forkCtx)
		expect(p?.cur).toBe(1)
		expect(p?.state).toBe('ahead')
	})

	it('conflict: current = Fork (stage 1), state conflict', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		const p = pipelineOf(item, forkCtx)
		expect(p?.cur).toBe(1)
		expect(p?.state).toBe('conflict')
	})

	it('done: current = last stage, state deployed', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		const p = pipelineOf(item, forkCtx)
		expect(p?.cur).toBe(2)
		expect(p?.state).toBe('deployed')
	})

	it('main context is a 2-stage Draft→parent pipeline', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: mainCtx })
		expect(pipelineOf(item, mainCtx)?.stages).toHaveLength(2)
	})

	it('bare (clean in-sync) row has no pipeline', () => {
		const bare = { local: false, parent: 'sync', done: false, removed: false } as DeployItem
		expect(pipelineOf(bare, forkCtx)).toBeUndefined()
	})
})

// ── action ──────────────────────────────────────────────────────────────────

describe('actionFor', () => {
	it('fork draft → Deploy to fork + discard secondary', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: forkCtx })
		const a = actionFor(item, forkCtx)
		expect(a.op).toBe('deploy_draft')
		expect(a.label).toBe('Deploy to fork')
		expect(a.targetStage).toBe('fork')
		expect(a.secondary?.[0].op).toBe('discard')
	})

	it('ahead → Deploy to <parent name>', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		const a = actionFor(item, forkCtx)
		expect(a.op).toBe('deploy_item')
		expect(a.label).toBe('Deploy to test-ai-sessions')
		expect(a.targetStage).toBe('parent')
	})

	it('conflict → no action (no safe one-click resolve; reconciled manually)', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(actionFor(item, forkCtx).op).toBe('none')
	})

	it('conflict has no action even when the item also has a pending draft', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/cd', { ahead: 1, behind: 1 })]),
			draftItems: [draft('flow', 'u/a/cd')],
			context: forkCtx
		})
		expect(item.local).toBe(true)
		expect(actionFor(item, forkCtx).op).toBe('none')
	})

	it('removed (ahead) → Delete in <parent>', () => {
		const [item] = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/gone', { ahead: 1, exists_in_fork: false, exists_in_source: true })
			]),
			draftItems: [],
			context: forkCtx
		})
		const a = actionFor(item, forkCtx)
		expect(a.op).toBe('delete_in_parent')
		expect(a.label).toBe('Delete in test-ai-sessions')
	})

	it('main draft deploys straight to parent', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: mainCtx })
		expect(actionFor(item, mainCtx).targetStage).toBe('parent')
	})

	it('falls back to "parent" when no name is given', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(actionFor(item, { ...forkCtx, parentName: undefined }).label).toBe('Deploy to parent')
	})

	it('done → op none', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(actionFor(item, forkCtx).op).toBe('none')
	})
})

// ── diff base ───────────────────────────────────────────────────────────────

describe('diffBaseFor', () => {
	it('draft row → deployed↔draft base', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f', { draft_only: true })],
			context: forkCtx
		})
		const b = diffBaseFor(item, forkCtx)
		expect(b.kind).toBe('draft')
		if (b.kind === 'draft') {
			expect(b.draftKind).toBe('script')
			expect(b.draftOnly).toBe(true)
			expect(b.workspaceId).toBe('wm-fork-f')
		}
	})

	it('ahead row → fork↔parent base carrying both workspace ids', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		const b = diffBaseFor(item, forkCtx)
		expect(b.kind).toBe('fork_parent')
		if (b.kind === 'fork_parent') {
			expect(b.forkWorkspaceId).toBe('wm-fork-f')
			expect(b.parentWorkspaceId).toBe('parent')
		}
	})

	it('terminal (done) row → self base in the session workspace', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		const b = diffBaseFor(item, forkCtx)
		expect(b.kind).toBe('self')
		if (b.kind === 'self') {
			expect(b.workspaceId).toBe('wm-fork-f')
			expect(b.path).toBe('u/a/done')
		}
	})
})

// ── on-behalf eligibility ───────────────────────────────────────────────────

describe('isOnBehalfEligible', () => {
	it('runnables and triggers/schedules are eligible; resources are not', () => {
		expect(isOnBehalfEligible('script')).toBe(true)
		expect(isOnBehalfEligible('raw_app')).toBe(true)
		expect(isOnBehalfEligible('schedule')).toBe(true)
		expect(isOnBehalfEligible('http_trigger')).toBe(true)
		expect(isOnBehalfEligible('resource')).toBe(false)
		expect(isOnBehalfEligible('variable')).toBe(false)
	})
})

// ── badgeCounts (bar readout) ─────────────────────────────────────────────────

describe('badgeCounts', () => {
	const items = buildDeployItems({
		comparison: comparison([
			diff('script', 'u/a/ahead', { ahead: 1 }),
			diff('flow', 'u/a/conflict', { ahead: 1, behind: 1 })
		]),
		draftItems: [draft('script', 'u/a/draft', { draft_only: true })],
		mask: new Set([
			maskKey('script', 'u/a/ahead'),
			maskKey('flow', 'u/a/conflict'),
			maskKey('script', 'u/a/draft'),
			maskKey('script', 'u/a/done')
		]),
		existingKeys: new Set([maskKey('script', 'u/a/done')]),
		context: forkCtx
	})

	it('tallies one bucket per item, matching badgeOf', () => {
		const counts = badgeCounts(items)
		expect(counts).toEqual({ conflict: 1, draft: 1, ahead: 1, deployed: 1, none: 0 })
	})
})

// ── deploy plan ─────────────────────────────────────────────────────────────

describe('deployPlanFor', () => {
	it('draft → deploy_draft within the current workspace', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('raw_app', 'u/a/app', { raw_app: true, draft_only: true })],
			context: forkCtx
		})
		const p = deployPlanFor(item, forkCtx)!
		expect(p.op).toBe('deploy_draft')
		expect(p.workspaceFrom).toBeUndefined()
		expect(p.rawApp).toBe(true)
		expect(p.draftOnly).toBe(true)
	})

	it('ahead → deploy_item fork→parent', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		const p = deployPlanFor(item, forkCtx)!
		expect(p.op).toBe('deploy_item')
		expect(p.workspaceFrom).toBe('wm-fork-f')
		expect(p.workspaceTo).toBe('parent')
	})

	it('conflict → no plan (reconciled manually, never auto-deployed)', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(item.parent).toBe('conflict')
		expect(deployPlanFor(item, forkCtx)).toBeUndefined()
	})

	it('removal → delete_in_parent targeting the parent', () => {
		const [item] = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/gone', { ahead: 1, exists_in_fork: false, exists_in_source: true })
			]),
			draftItems: [],
			context: forkCtx
		})
		const p = deployPlanFor(item, forkCtx)!
		expect(p.op).toBe('delete_in_parent')
		expect(p.workspaceTo).toBe('parent')
	})

	it('terminal (done) rows have no plan', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(item.done).toBe(true)
		expect(deployPlanFor(item, forkCtx)).toBeUndefined()
	})

	it('discardPlanFor only applies to drafts and carries the legacy flag', () => {
		const [d] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f', { legacy_draft: true })],
			context: forkCtx
		})
		const p = discardPlanFor(d, forkCtx)!
		expect(p.op).toBe('discard')
		expect(p.legacy).toBe(true)
		expect(p.workspaceTo).toBe('wm-fork-f')

		const [ahead] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/g', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(discardPlanFor(ahead, forkCtx)).toBeUndefined()
	})
})
