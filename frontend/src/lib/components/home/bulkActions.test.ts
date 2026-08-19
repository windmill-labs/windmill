import { describe, it, expect } from 'vitest'
import { blockedReason, movedPath, type BulkContext } from './bulkActions'
import type { BulkItem } from './homeSelection.svelte'

/** A fully-permitted, deployed row; each case overrides only what it is about. */
function item(over: Partial<BulkItem> = {}): BulkItem {
	return {
		key: 'script/f/alpha/x',
		kind: 'script',
		path: 'f/alpha/x',
		displayPath: 'f/alpha/x',
		summary: '',
		canWrite: true,
		owner: true,
		archived: false,
		draftOnly: false,
		isDraft: false,
		rawApp: false,
		...over
	}
}

const admin: BulkContext = { workspace: 'w', isAdmin: true }
const member: BulkContext = { workspace: 'w', isAdmin: false }

// Each action's gate has to keep matching what the equivalent per-row menu entry
// allows (common/table/{Script,Flow,App}Row.svelte, MoveDrawer.svelte) — a bulk
// action that drifts looser than the single-row one is a permission hole.
describe('blockedReason', () => {
	it('lets a fully-permitted deployed row through every action it applies to', () => {
		expect(blockedReason('move', item(), admin)).toBeUndefined()
		expect(blockedReason('archive', item(), admin)).toBeUndefined()
		expect(blockedReason('delete', item(), admin)).toBeUndefined()
		expect(blockedReason('unarchive', item({ archived: true }), admin)).toBeUndefined()
		expect(blockedReason('discard', item({ isDraft: true }), admin)).toBeUndefined()
	})

	it('gates delete per kind: script needs admin, flow needs owner, app needs write', () => {
		expect(blockedReason('delete', item({ kind: 'script' }), member)).toBeDefined()
		expect(blockedReason('delete', item({ kind: 'flow', owner: false }), member)).toBeDefined()
		expect(blockedReason('delete', item({ kind: 'flow' }), member)).toBeUndefined()
		// An app is deletable without ownership, matching AppRow's `disabled: !canEdit`.
		expect(blockedReason('delete', item({ kind: 'app', owner: false }), member)).toBeUndefined()
		expect(blockedReason('delete', item({ kind: 'app', canWrite: false }), member)).toBeDefined()
	})

	it('requires ownership to move, and refuses archived rows', () => {
		expect(blockedReason('move', item({ owner: false }), admin)).toBeDefined()
		expect(blockedReason('move', item({ canWrite: false }), admin)).toBeDefined()
		expect(blockedReason('move', item({ archived: true }), admin)).toBeDefined()
	})

	it('archives only scripts and flows, and only in the matching state', () => {
		expect(blockedReason('archive', item({ kind: 'app' }), admin)).toBeDefined()
		expect(blockedReason('archive', item({ archived: true }), admin)).toBeDefined()
		expect(blockedReason('unarchive', item({ archived: false }), admin)).toBeDefined()
	})

	it('routes a draft-only row to discard, never to move/archive/delete', () => {
		const draftOnly = item({ draftOnly: true, isDraft: true })
		expect(blockedReason('discard', draftOnly, admin)).toBeUndefined()
		expect(blockedReason('move', draftOnly, admin)).toBeDefined()
		expect(blockedReason('archive', draftOnly, admin)).toBeDefined()
		expect(blockedReason('delete', draftOnly, admin)).toBeDefined()
	})

	it('has nothing to discard on a row with no draft of yours', () => {
		expect(blockedReason('discard', item(), admin)).toBeDefined()
	})
})

describe('movedPath', () => {
	it('keeps everything below the owner prefix, so nested paths keep their shape', () => {
		expect(movedPath(item({ path: 'f/alpha/x' }), 'f/beta')).toBe('f/beta/x')
		expect(movedPath(item({ path: 'f/alpha/sub/x' }), 'f/beta')).toBe('f/beta/sub/x')
		expect(movedPath(item({ path: 'u/ana/x' }), 'f/beta')).toBe('f/beta/x')
	})
})
