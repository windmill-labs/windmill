import { describe, it, expect } from 'vitest'
import type { UserDraftItemKind } from '$lib/gen'
import type { DraftItem } from '$lib/workspaceDrafts.svelte'
import {
	buildDeployItems,
	maskOnlyCandidates,
	badgeOf,
	actionFor,
	diffBaseFor,
	badgeCounts,
	deployPlanFor,
	discardPlanFor
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

// ── buildDeployItems ─────────────────────────────────────────────────────────

describe('buildDeployItems', () => {
	it('a draft is a live (not done) row carrying the draft flags', () => {
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/foo', { draft_only: true })]
		})
		expect(items).toHaveLength(1)
		expect(items[0].hasDraft).toBe(true)
		expect(items[0].done).toBe(false)
		expect(items[0].draftOnly).toBe(true)
		expect(items[0].key).toBe(maskKey('script', 'u/a/foo'))
	})

	it('maps the trigger taxonomy to the deploy kind (trigger_schedule → schedule)', () => {
		const items = buildDeployItems({ draftItems: [draft('trigger_schedule', 'u/a/sched')] })
		expect(items[0].deployKind).toBe('schedule')
		expect(items[0].draftKind).toBe('trigger_schedule')
	})

	it('maps both email draft kinds to the email_trigger deploy kind', () => {
		const items = buildDeployItems({
			draftItems: [draft('trigger_email', 'u/a/em'), draft('trigger_default_email', 'u/a/dem')]
		})
		expect(items.map((i) => i.deployKind)).toEqual(['email_trigger', 'email_trigger'])
	})

	it('mask scopes drafts to touched items', () => {
		const mask = new Set([maskKey('script', 'u/a/keep')])
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/keep'), draft('flow', 'u/a/other')],
			mask
		})
		expect(items.map((i) => i.path)).toEqual(['u/a/keep'])
	})

	it('mask-only entries that still exist surface as terminal deployed rows', () => {
		const key = maskKey('script', 'u/a/done')
		const items = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key])
		})
		expect(items).toHaveLength(1)
		expect(items[0].done).toBe(true)
		expect(items[0].hasDraft).toBe(false)
		expect(items[0].path).toBe('u/a/done')
	})

	it('a mask-only entry that no longer exists (discarded or deleted) is dropped', () => {
		const key = maskKey('script', 'u/a/gone')
		// Existence unresolved → no mask-only rows yet.
		expect(buildDeployItems({ draftItems: [], mask: new Set([key]) })).toHaveLength(0)
		// Existence resolved to gone → dropped.
		expect(
			buildDeployItems({ draftItems: [], mask: new Set([key]), existingKeys: new Set() })
		).toHaveLength(0)
	})

	it('a draft covers its mask entry (no duplicate deployed row)', () => {
		const key = maskKey('script', 'u/a/foo')
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/foo')],
			mask: new Set([key]),
			existingKeys: new Set([key])
		})
		expect(items).toHaveLength(1)
		expect(items[0].hasDraft).toBe(true)
	})

	it('maskOnlyCandidates returns keys not covered by a draft', () => {
		const cands = maskOnlyCandidates({
			draftItems: [draft('script', 'u/a/draft')],
			mask: new Set([maskKey('script', 'u/a/draft'), maskKey('trigger_schedule', 'u/a/done')])
		})
		expect(cands.map((c) => c.key)).toEqual([maskKey('trigger_schedule', 'u/a/done')])
		expect(cands[0].deployKind).toBe('schedule')
	})

	it('no mask lists every draft', () => {
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/x'), draft('flow', 'u/a/y')]
		})
		expect(items).toHaveLength(2)
	})
})

// ── badge ────────────────────────────────────────────────────────────────────

describe('badgeOf / badgeCounts', () => {
	it('draft row → draft; deployed row → deployed', () => {
		const key = maskKey('script', 'u/a/done')
		const items = buildDeployItems({
			draftItems: [draft('script', 'u/a/d')],
			mask: new Set([maskKey('script', 'u/a/d'), key]),
			existingKeys: new Set([key])
		})
		const by = new Map(items.map((i) => [i.path, i]))
		expect(badgeOf(by.get('u/a/d')!)).toBe('draft')
		expect(badgeOf(by.get('u/a/done')!)).toBe('deployed')
		expect(badgeCounts(items)).toEqual({ draft: 1, deployed: 1 })
	})
})

// ── action ──────────────────────────────────────────────────────────────────

describe('actionFor', () => {
	it('draft → Deploy + discard secondary', () => {
		const [item] = buildDeployItems({ draftItems: [draft('script', 'u/a/f')] })
		const a = actionFor(item)
		expect(a.op).toBe('deploy_draft')
		expect(a.label).toBe('Deploy')
		expect(a.secondary?.[0].op).toBe('discard')
	})

	it('draft-only discard is labelled "Discard draft"', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f', { draft_only: true })]
		})
		expect(actionFor(item).secondary?.[0].label).toBe('Discard draft')
	})

	it('deployed (done) → op none', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key])
		})
		expect(actionFor(item).op).toBe('none')
	})
})

// ── diff base ───────────────────────────────────────────────────────────────

describe('diffBaseFor', () => {
	it('draft row → deployed↔draft base', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f', { draft_only: true })]
		})
		const b = diffBaseFor(item, 'ws')
		expect(b.kind).toBe('draft')
		if (b.kind === 'draft') {
			expect(b.draftKind).toBe('script')
			expect(b.draftOnly).toBe(true)
			expect(b.workspaceId).toBe('ws')
		}
	})

	it('deployed row → self base in the session workspace', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key])
		})
		const b = diffBaseFor(item, 'ws')
		expect(b.kind).toBe('self')
		if (b.kind === 'self') {
			expect(b.workspaceId).toBe('ws')
			expect(b.path).toBe('u/a/done')
		}
	})
})

// ── deploy plan ─────────────────────────────────────────────────────────────

describe('deployPlanFor / discardPlanFor', () => {
	it('draft → deploy_draft carrying rawApp/draftOnly', () => {
		const [item] = buildDeployItems({
			draftItems: [draft('raw_app', 'u/a/app', { raw_app: true, draft_only: true })]
		})
		const p = deployPlanFor(item)!
		expect(p.op).toBe('deploy_draft')
		expect(p.rawApp).toBe(true)
		expect(p.draftOnly).toBe(true)
	})

	it('deployed (done) rows have no plan', () => {
		const key = maskKey('script', 'u/a/done')
		const [item] = buildDeployItems({
			draftItems: [],
			mask: new Set([key]),
			existingKeys: new Set([key])
		})
		expect(item.done).toBe(true)
		expect(deployPlanFor(item)).toBeUndefined()
		expect(discardPlanFor(item)).toBeUndefined()
	})

	it('discardPlanFor carries the legacy flag', () => {
		const [d] = buildDeployItems({
			draftItems: [draft('script', 'u/a/f', { legacy_draft: true })]
		})
		const p = discardPlanFor(d)!
		expect(p.op).toBe('discard')
		expect(p.legacy).toBe(true)
	})
})
