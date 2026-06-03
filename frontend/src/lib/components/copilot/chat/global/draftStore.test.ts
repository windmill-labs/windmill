import { beforeEach, describe, expect, it, vi } from 'vitest'

// draftStore.ts is the single precedence point over the DB layer (`./dbDraft`)
// and the localStorage layer (`./userDraftAdapter` + `$lib/userDraft.svelte`).
// Mock all three so the orchestration is exercised in isolation.
vi.mock('./dbDraft', () => ({
	saveScriptDbDraft: vi.fn(async () => {}),
	saveFlowDbDraft: vi.fn(async () => {}),
	saveAppDbDraft: vi.fn(async () => {}),
	readScriptDbDraft: vi.fn(async () => {
		throw new Error('readScriptDbDraft mock not configured')
	}),
	readFlowDbDraft: vi.fn(async () => {
		throw new Error('readFlowDbDraft mock not configured')
	}),
	readAppDbDraft: vi.fn(async () => {
		throw new Error('readAppDbDraft mock not configured')
	})
}))

vi.mock('./userDraftAdapter', () => ({
	getGlobalDraftStoragePath: vi.fn((_ws: string, _type: string, path: string) => path),
	saveGlobalAppDraft: vi.fn(),
	saveGlobalScriptDraft: vi.fn(),
	saveGlobalFlowDraft: vi.fn(),
	userDraftKindFor: vi.fn((type: string) =>
		type === 'app' ? 'raw_app' : type === 'script' || type === 'flow' ? type : undefined
	)
}))

vi.mock('$lib/userDraft.svelte', () => ({
	UserDraft: {
		getLiveEditorDraft: vi.fn(() => undefined),
		get: vi.fn(() => undefined),
		save: vi.fn()
	}
}))

import {
	saveScriptDbDraft,
	saveFlowDbDraft,
	saveAppDbDraft,
	readScriptDbDraft,
	readFlowDbDraft,
	readAppDbDraft
} from './dbDraft'
import {
	getGlobalDraftStoragePath,
	saveGlobalAppDraft,
	saveGlobalScriptDraft,
	saveGlobalFlowDraft
} from './userDraftAdapter'
import { UserDraft } from '$lib/userDraft.svelte'
import { hasLiveEditor, loadDraft, persistDraft } from './draftStore'

const ws = 'test-ws'

const readScript = readScriptDbDraft as unknown as ReturnType<typeof vi.fn>
const readFlow = readFlowDbDraft as unknown as ReturnType<typeof vi.fn>
const readApp = readAppDbDraft as unknown as ReturnType<typeof vi.fn>
const saveScript = saveScriptDbDraft as unknown as ReturnType<typeof vi.fn>
const saveFlow = saveFlowDbDraft as unknown as ReturnType<typeof vi.fn>
const saveApp = saveAppDbDraft as unknown as ReturnType<typeof vi.fn>
const storagePathMock = getGlobalDraftStoragePath as unknown as ReturnType<typeof vi.fn>
const saveAppMirror = saveGlobalAppDraft as unknown as ReturnType<typeof vi.fn>
const saveScriptMirror = saveGlobalScriptDraft as unknown as ReturnType<typeof vi.fn>
const saveFlowMirror = saveGlobalFlowDraft as unknown as ReturnType<typeof vi.fn>
const getLiveEditorDraft = UserDraft.getLiveEditorDraft as unknown as ReturnType<typeof vi.fn>
const userDraftGet = UserDraft.get as unknown as ReturnType<typeof vi.fn>

// Helpers to drive the "is a live editor open for this path" flag.
function openLiveEditor(storagePath: string, effectivePath?: string) {
	getLiveEditorDraft.mockImplementation(() => ({ storagePath, effectivePath }))
}
function noLiveEditor() {
	getLiveEditorDraft.mockImplementation(() => undefined)
}

beforeEach(() => {
	vi.clearAllMocks()
	storagePathMock.mockImplementation((_ws: string, _type: string, path: string) => path)
	noLiveEditor()
	userDraftGet.mockImplementation(() => undefined)
})

describe('hasLiveEditor', () => {
	it('false when no live editor is open', () => {
		noLiveEditor()
		expect(hasLiveEditor(ws, 'script', 'u/me/foo')).toBe(false)
	})

	it('true when a live editor is open for this (type, path)', () => {
		openLiveEditor('u/me/foo')
		expect(hasLiveEditor(ws, 'script', 'u/me/foo')).toBe(true)
	})

	it('matches on the effectivePath too', () => {
		openLiveEditor('draft-storage-path', 'u/me/foo')
		// getGlobalDraftStoragePath would resolve u/me/foo -> the live storagePath.
		storagePathMock.mockImplementation(() => 'draft-storage-path')
		expect(hasLiveEditor(ws, 'script', 'u/me/foo')).toBe(true)
	})

	it('false when a live editor is open for a different path', () => {
		openLiveEditor('u/me/other')
		expect(hasLiveEditor(ws, 'script', 'u/me/foo')).toBe(false)
	})

	it('false for non Group-A types', () => {
		openLiveEditor('u/me/foo')
		expect(hasLiveEditor(ws, 'variable' as any, 'u/me/foo')).toBe(false)
	})
})

describe('loadDraft - no live editor (DB is source of truth)', () => {
	it('script: returns the DB draft value + flags when a DB draft exists', async () => {
		noLiveEditor()
		readScript.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/foo', content: 'draft content', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1', remoteDraftRev: 'd1' }
		})
		const res = await loadDraft('script', 'u/me/foo', ws)
		expect(res.source).toBe('db')
		expect((res.value as any)?.content).toBe('draft content')
		expect(res.itemExists).toBe(true)
		expect(res.hasDbDraft).toBe(true)
		expect(res.meta).toEqual({ remoteRev: 'h1', remoteDraftRev: 'd1' })
		// No localStorage read on the DB branch.
		expect(userDraftGet).not.toHaveBeenCalled()
	})

	it('script: returns the deployed value when no DB draft exists', async () => {
		noLiveEditor()
		readScript.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: false,
			value: { path: 'u/me/foo', content: 'deployed', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1' }
		})
		const res = await loadDraft('script', 'u/me/foo', ws)
		expect(res.source).toBe('deployed')
		expect((res.value as any)?.content).toBe('deployed')
	})

	it('script: returns none when the item is not found', async () => {
		noLiveEditor()
		readScript.mockResolvedValueOnce({
			itemExists: false,
			deployedExists: false,
			draftOnly: false,
			hasDbDraft: false,
			value: undefined,
			meta: {}
		})
		const res = await loadDraft('script', 'u/me/missing', ws)
		expect(res.source).toBe('none')
		expect(res.value).toBeUndefined()
		expect(res.itemExists).toBe(false)
	})

	it('flow: reads from the DB layer', async () => {
		noLiveEditor()
		readFlow.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/flow', value: { modules: [{ id: 'a' }] } },
			meta: { remoteRev: 7, remoteDraftRev: 'd2' }
		})
		const res = await loadDraft('flow', 'u/me/flow', ws)
		expect(res.source).toBe('db')
		expect((res.value as any)?.value).toEqual({ modules: [{ id: 'a' }] })
		expect(readFlow).toHaveBeenCalledWith(ws, 'u/me/flow')
	})

	it('app: reads from the DB layer (AppDraftValue shape)', async () => {
		noLiveEditor()
		readApp.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { summary: 's', files: { '/a.tsx': 'x' }, runnables: {}, data: { tables: [] } },
			meta: { remoteRev: 4, remoteDraftRev: 'd3' }
		})
		const res = await loadDraft('app', 'u/me/app', ws)
		expect(res.source).toBe('db')
		expect((res.value as any)?.files).toEqual({ '/a.tsx': 'x' })
	})
})

describe('loadDraft - live editor open (localStorage mirror wins)', () => {
	it('script: returns the raw localStorage NewScript when a live editor + localStorage draft exist', async () => {
		openLiveEditor('u/me/foo')
		const localValue = { path: 'u/me/foo', content: 'LIVE EDIT', language: 'bun', kind: 'script' }
		userDraftGet.mockImplementation(() => localValue)
		const res = await loadDraft('script', 'u/me/foo', ws)
		expect(res.source).toBe('live')
		expect((res.value as any)?.content).toBe('LIVE EDIT')
		// The live read must use the per-type itemKind + the live storage path.
		expect(userDraftGet).toHaveBeenCalledWith('script', 'u/me/foo', { workspace: ws })
		// A pure live read does NOT need the DB call.
		expect(readScript).not.toHaveBeenCalled()
	})

	it('app: returns the raw localStorage AppDraftValue on the live branch', async () => {
		openLiveEditor('u/me/app')
		const localValue = {
			summary: 'live',
			files: { '/a.tsx': 'LIVE' },
			runnables: {},
			data: { tables: [] }
		}
		userDraftGet.mockImplementation(() => localValue)
		const res = await loadDraft('app', 'u/me/app', ws)
		expect(res.source).toBe('live')
		expect((res.value as any)?.files).toEqual({ '/a.tsx': 'LIVE' })
		expect(userDraftGet).toHaveBeenCalledWith('raw_app', 'u/me/app', { workspace: ws })
		expect(readApp).not.toHaveBeenCalled()
	})

	it('falls back to the DB draft when a live editor is open but has no localStorage entry', async () => {
		openLiveEditor('u/me/foo')
		userDraftGet.mockImplementation(() => undefined)
		readScript.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/foo', content: 'db draft', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1', remoteDraftRev: 'd1' }
		})
		const res = await loadDraft('script', 'u/me/foo', ws)
		expect(res.source).toBe('db')
		expect((res.value as any)?.content).toBe('db draft')
	})

	it('ignores a stale localStorage entry when NO live editor is open (DB is truth)', async () => {
		noLiveEditor()
		userDraftGet.mockImplementation(() => ({ content: 'STALE LOCAL', language: 'bun' }))
		readScript.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/foo', content: 'db draft', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1', remoteDraftRev: 'd1' }
		})
		const res = await loadDraft('script', 'u/me/foo', ws)
		expect(res.source).toBe('db')
		expect((res.value as any)?.content).toBe('db draft')
	})
})

describe('persistDraft - no live editor (DB only)', () => {
	it('script: writes the DB draft and does NOT mirror to localStorage', async () => {
		noLiveEditor()
		// itemExists provided -> no extra read.
		await persistDraft('script', 'u/me/foo', ws, {
			path: 'u/me/foo',
			content: 'c',
			language: 'bun',
			kind: 'script'
		} as any, { itemExists: true })

		expect(saveScript).toHaveBeenCalledTimes(1)
		expect(saveScript.mock.calls[0][3]).toEqual({ itemExists: true })
		expect(saveScriptMirror).not.toHaveBeenCalled()
		expect(UserDraft.save).not.toHaveBeenCalled()
		// itemExists was provided -> no read.
		expect(readScript).not.toHaveBeenCalled()
	})

	it('reads itemExists once when not provided, then threads it into save*', async () => {
		noLiveEditor()
		readScript.mockResolvedValueOnce({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: false,
			value: { path: 'u/me/foo', content: 'deployed', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1' }
		})
		await persistDraft('script', 'u/me/foo', ws, {
			path: 'u/me/foo',
			content: 'c',
			language: 'bun',
			kind: 'script'
		} as any)
		expect(readScript).toHaveBeenCalledTimes(1)
		expect(saveScript).toHaveBeenCalledTimes(1)
		expect(saveScript.mock.calls[0][3]).toEqual({ itemExists: true })
	})
})

describe('persistDraft - live editor open (DB write + localStorage mirror)', () => {
	it('script: writes the DB draft AND mirrors to localStorage with fresh meta', async () => {
		openLiveEditor('u/me/foo')
		// The post-write read provides the fresh remoteDraftRev for the mirror (option a).
		readScript.mockResolvedValue({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/foo', content: 'c', language: 'bun', kind: 'script' },
			meta: { remoteRev: 'h1', remoteDraftRev: 'fresh-draft-rev' }
		})
		const draft = { path: 'u/me/foo', content: 'c', language: 'bun', kind: 'script' } as any
		await persistDraft('script', 'u/me/foo', ws, draft, { itemExists: true })

		expect(saveScript).toHaveBeenCalledTimes(1)
		expect(saveScriptMirror).toHaveBeenCalledTimes(1)
		const [mws, mpath, mvalue, mmeta] = saveScriptMirror.mock.calls[0]
		expect(mws).toBe(ws)
		expect(mpath).toBe('u/me/foo')
		expect((mvalue as any).content).toBe('c')
		// Mirror stamped with the fresh DB-draft rev so the editor banner isn't stale.
		expect(mmeta).toEqual({ remoteRev: 'h1', remoteDraftRev: 'fresh-draft-rev' })
	})

	it('app: mirrors via saveGlobalAppDraft', async () => {
		openLiveEditor('u/me/app')
		readApp.mockResolvedValue({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { summary: 's', files: {}, runnables: {}, data: { tables: [] } },
			meta: { remoteRev: 4, remoteDraftRev: 'fresh-app-rev' }
		})
		const value = { summary: 's', files: { '/a.tsx': 'x' }, runnables: {}, data: { tables: [] } } as any
		await persistDraft('app', 'u/me/app', ws, value, { itemExists: true })
		expect(saveApp).toHaveBeenCalledTimes(1)
		expect(saveAppMirror).toHaveBeenCalledTimes(1)
		const [, , mvalue, mmeta] = saveAppMirror.mock.calls[0]
		expect((mvalue as any).files).toEqual({ '/a.tsx': 'x' })
		expect(mmeta).toEqual({ remoteRev: 4, remoteDraftRev: 'fresh-app-rev' })
	})

	it('flow: writes the DB draft AND mirrors', async () => {
		openLiveEditor('u/me/flow')
		readFlow.mockResolvedValue({
			itemExists: true,
			deployedExists: true,
			draftOnly: false,
			hasDbDraft: true,
			value: { path: 'u/me/flow', value: { modules: [] } },
			meta: { remoteRev: 9, remoteDraftRev: 'fresh-flow-rev' }
		})
		const draft = { path: 'u/me/flow', value: { modules: [] } } as any
		await persistDraft('flow', 'u/me/flow', ws, draft, { itemExists: true })
		expect(saveFlow).toHaveBeenCalledTimes(1)
		expect(saveFlowMirror).toHaveBeenCalledTimes(1)
		const [, , , mmeta] = saveFlowMirror.mock.calls[0]
		expect(mmeta).toEqual({ remoteRev: 9, remoteDraftRev: 'fresh-flow-rev' })
	})
})
