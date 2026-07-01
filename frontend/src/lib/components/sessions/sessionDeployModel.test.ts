import { describe, it, expect } from 'vitest'
import type { UserDraftItemKind, WorkspaceComparison, WorkspaceItemDiff } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	buildDeployItems,
	maskOnlyCandidates,
	pipelineOf,
	actionFor,
	diffBaseFor,
	isOnBehalfEligible,
	selectableOf,
	defaultSelection,
	inSegment,
	segmentCounts,
	footerSummary,
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

// ── buildDeployItems: merge + state derivation ──────────────────────────────

describe('buildDeployItems — state derivation', () => {
	it('a draft-only item is state=draft, exists nowhere yet', () => {
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/foo', { draft_only: true })],
			context: mainCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].state).toBe('draft')
		expect(items[0].draftOnly).toBe(true)
		expect(items[0].existsInParent).toBe(false)
	})

	it('a fork diff ahead-only (no draft) is state=in_fork', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/foo', { ahead: 2, behind: 0 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].state).toBe('in_fork')
		expect(items[0].hasDraft).toBe(false)
		expect(items[0].ahead).toBe(2)
	})

	it('ahead AND behind is state=conflict', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/bar', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].state).toBe('conflict')
	})

	it('removed in fork (present in parent) is state=deleted', () => {
		const items = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/gone', { ahead: 1, exists_in_fork: false, exists_in_source: true })
			]),
			draftItems: [],
			context: forkCtx
		})
		expect(items[0].state).toBe('deleted')
	})

	it('a pending draft on a deployed fork item takes precedence → state=draft, merged to ONE row', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/foo', { ahead: 1, behind: 0 })]),
			draftItems: [draft('script', 'u/a/foo')],
			context: forkCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].state).toBe('draft')
		expect(items[0].hasDraft).toBe(true)
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
		expect(items[0].hasDraft).toBe(true)
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

	it('mask-only entries that still exist surface as terminal in_parent rows', () => {
		const key = maskKey('script', 'u/a/done')
		const items = buildDeployItems({
			comparison: comparison([]),
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(items).toHaveLength(1)
		expect(items[0].state).toBe('in_parent')
		expect(items[0].path).toBe('u/a/done')
	})

	it('a mask-only entry that no longer exists (discarded draft) is dropped, not shown as in_parent', () => {
		const key = maskKey('script', 'u/a/gone')
		// existingKeys omitted (existence unresolved) → no terminal row
		const pending = buildDeployItems({
			comparison: comparison([]),
			draftItems: [],
			mask: new Set([key]),
			context: forkCtx
		})
		expect(pending).toHaveLength(0)
		// existence resolved and the item is absent → still dropped
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
		// mapped to the deploy kind for the existence check
		expect(cands[0].deployKind).toBe('schedule')
	})

	it('main (non-fork) context ignores comparison and lists drafts only', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/forkonly')]),
			draftItems: [draft('script', 'u/a/d', { draft_only: true })],
			context: mainCtx
		})
		expect(items.map((i) => i.path)).toEqual(['u/a/d'])
		expect(items[0].state).toBe('draft')
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

// ── pipeline ────────────────────────────────────────────────────────────────

describe('pipelineOf', () => {
	function stage(item: DeployItem, ctx: SessionContext, id: string) {
		return pipelineOf(item, ctx).find((s) => s.id === id)?.status
	}

	it('fork draft: draft current, fork+parent todo', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f')],
			context: forkCtx
		})
		expect(pipelineOf(item, forkCtx).map((s) => s.id)).toEqual(['draft', 'fork', 'parent'])
		expect(stage(item, forkCtx, 'draft')).toBe('current')
		expect(stage(item, forkCtx, 'fork')).toBe('todo')
	})

	it('in_fork: draft done, fork current', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f')]),
			draftItems: [],
			context: forkCtx
		})
		expect(stage(item, forkCtx, 'draft')).toBe('done')
		expect(stage(item, forkCtx, 'fork')).toBe('current')
		expect(stage(item, forkCtx, 'parent')).toBe('todo')
	})

	it('conflict blocks the fork stage', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(stage(item, forkCtx, 'fork')).toBe('blocked')
	})

	it('main context is a 2-stage draft→parent pipeline', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')], context: mainCtx })
		expect(pipelineOf(item, mainCtx).map((s) => s.id)).toEqual(['draft', 'parent'])
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

	it('in_fork → Deploy to <parent name>', () => {
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

	it('behind-only in_fork → Update fork', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 0, behind: 2 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(actionFor(item, forkCtx).op).toBe('update_fork')
	})

	it('conflict → op none with a blocked reason', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		const a = actionFor(item, forkCtx)
		expect(a.op).toBe('none')
		expect(a.blockedReason).toBeTruthy()
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

	it('in_fork row → fork↔parent base carrying both workspace ids', () => {
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

	it('terminal (in_parent) row → no diff', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key]),
			context: forkCtx
		})
		expect(diffBaseFor(item, forkCtx).kind).toBe('none')
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

// ── selection ───────────────────────────────────────────────────────────────

describe('selection', () => {
	it('conflicts and done rows are not selectable; drafts need own+write', () => {
		const items = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/infork', { ahead: 1 }),
				diff('flow', 'u/a/conflict', { ahead: 1, behind: 1 })
			]),
			draftItems: [
				draft('script', 'u/a/mine'),
				draft('script', 'u/a/theirs', { mine: false }),
				draft('script', 'u/a/readonly', { can_write: false })
			],
			mask: new Set([
				maskKey('script', 'u/a/infork'),
				maskKey('flow', 'u/a/conflict'),
				maskKey('script', 'u/a/mine'),
				maskKey('script', 'u/a/theirs'),
				maskKey('script', 'u/a/readonly'),
				maskKey('script', 'u/a/done') // mask-only → in_parent
			]),
			existingKeys: new Set([maskKey('script', 'u/a/done')]),
			context: forkCtx
		})
		const m = byKey(items)
		expect(selectableOf(m.get(maskKey('script', 'u/a/infork'))!)).toBe(true)
		expect(selectableOf(m.get(maskKey('flow', 'u/a/conflict'))!)).toBe(false)
		expect(selectableOf(m.get(maskKey('script', 'u/a/mine'))!)).toBe(true)
		expect(selectableOf(m.get(maskKey('script', 'u/a/theirs'))!)).toBe(false)
		expect(selectableOf(m.get(maskKey('script', 'u/a/readonly'))!)).toBe(false)
		expect(selectableOf(m.get(maskKey('script', 'u/a/done'))!)).toBe(false)
	})

	it('defaultSelection checks exactly the selectable rows', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/conflict', { ahead: 1, behind: 1 })]),
			draftItems: [draft('script', 'u/a/mine'), draft('script', 'u/a/theirs', { mine: false })],
			context: forkCtx
		})
		expect(new Set(defaultSelection(items))).toEqual(new Set([maskKey('script', 'u/a/mine')]))
	})
})

// ── segments ────────────────────────────────────────────────────────────────

describe('segments', () => {
	const items = buildDeployItems({
		comparison: comparison([diff('script', 'u/a/infork', { ahead: 1 })]),
		draftItems: [draft('script', 'u/a/draft', { draft_only: true })],
		mask: new Set([
			maskKey('script', 'u/a/infork'),
			maskKey('script', 'u/a/draft'),
			maskKey('script', 'u/a/done')
		]),
		existingKeys: new Set([maskKey('script', 'u/a/done')]),
		context: forkCtx
	})

	it('assigns each item to its lifecycle bucket', () => {
		const m = byKey(items)
		expect(inSegment(m.get(maskKey('script', 'u/a/draft'))!, 'drafts')).toBe(true)
		expect(inSegment(m.get(maskKey('script', 'u/a/infork'))!, 'in_fork')).toBe(true)
		expect(inSegment(m.get(maskKey('script', 'u/a/done'))!, 'done')).toBe(true)
	})

	it('to_review excludes done; all includes everything', () => {
		const counts = segmentCounts(items)
		expect(counts.all).toBe(3)
		expect(counts.done).toBe(1)
		expect(counts.to_review).toBe(2)
		expect(counts.drafts).toBe(1)
		expect(counts.in_fork).toBe(1)
	})
})

// ── footer ──────────────────────────────────────────────────────────────────

describe('footerSummary', () => {
	it('groups the selection by deploy target and counts on-behalf-eligible parent promotions', () => {
		const items = buildDeployItems({
			comparison: comparison([
				diff('script', 'u/a/toparent', { ahead: 1 }),
				diff('resource', 'u/a/res', { ahead: 1 })
			]),
			draftItems: [draft('script', 'u/a/tofork')],
			mask: new Set([
				maskKey('script', 'u/a/toparent'),
				maskKey('resource', 'u/a/res'),
				maskKey('script', 'u/a/tofork')
			]),
			context: forkCtx
		})
		const all = new Set(items.map((i) => i.key))
		const s = footerSummary(items, all, forkCtx)
		expect(s.toFork).toBe(1) // the draft
		expect(s.toParent).toBe(2) // script + resource in-fork promotions
		expect(s.onBehalfEligible).toBe(1) // only the script promotion, not the resource
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

	it('in_fork ahead → deploy_item fork→parent', () => {
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

	it('behind-only → update_fork parent→fork', () => {
		const [item] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/f', { ahead: 0, behind: 2 })]),
			draftItems: [],
			context: forkCtx
		})
		const p = deployPlanFor(item, forkCtx)!
		expect(p.op).toBe('update_fork')
		expect(p.workspaceFrom).toBe('parent')
		expect(p.workspaceTo).toBe('wm-fork-f')
	})

	it('deletion → delete_in_parent targeting the parent', () => {
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

	it('terminal and conflict rows have no plan', () => {
		const items = buildDeployItems({
			comparison: comparison([diff('flow', 'u/a/c', { ahead: 1, behind: 1 })]),
			draftItems: [],
			mask: new Set([maskKey('flow', 'u/a/c'), maskKey('script', 'u/a/done')]),
			existingKeys: new Set([maskKey('script', 'u/a/done')]),
			context: forkCtx
		})
		expect(items.some((i) => i.state === 'in_parent')).toBe(true)
		for (const item of items) expect(deployPlanFor(item, forkCtx)).toBeUndefined()
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

		const [inFork] = buildDeployItems({
			comparison: comparison([diff('script', 'u/a/g', { ahead: 1 })]),
			draftItems: [],
			context: forkCtx
		})
		expect(discardPlanFor(inFork, forkCtx)).toBeUndefined()
	})
})
