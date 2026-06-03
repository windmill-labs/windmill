import { beforeEach, describe, expect, it, vi } from 'vitest'

// dbDraft.ts only touches these services. Mock them as plain spies so the
// unit is exercised in isolation (no transitive $lib/gen imports).
vi.mock('$lib/gen', () => ({
	ScriptService: {
		createScript: vi.fn(async () => 'new-hash'),
		getScriptByPathWithDraft: vi.fn(async () => {
			throw new Error('getScriptByPathWithDraft mock not configured')
		})
	},
	FlowService: {
		createFlow: vi.fn(async () => 'created'),
		getFlowByPathWithDraft: vi.fn(async () => {
			throw new Error('getFlowByPathWithDraft mock not configured')
		}),
		getFlowLatestVersion: vi.fn(async () => {
			throw new Error('getFlowLatestVersion mock not configured')
		})
	},
	AppService: {
		createAppRaw: vi.fn(async () => 'created'),
		getAppByPathWithDraft: vi.fn(async () => {
			throw new Error('getAppByPathWithDraft mock not configured')
		})
	},
	DraftService: { createDraft: vi.fn(async () => 'created') }
}))

import { AppService, DraftService, FlowService, ScriptService } from '$lib/gen'
import {
	saveScriptDbDraft,
	saveFlowDbDraft,
	saveAppDbDraft,
	readScriptDbDraft,
	readFlowDbDraft,
	readAppDbDraft
} from './dbDraft'

const ws = 'test-ws'

const scriptCreate = ScriptService.createScript as unknown as ReturnType<typeof vi.fn>
const flowCreate = FlowService.createFlow as unknown as ReturnType<typeof vi.fn>
const appCreate = AppService.createAppRaw as unknown as ReturnType<typeof vi.fn>
const draftCreate = DraftService.createDraft as unknown as ReturnType<typeof vi.fn>
const scriptWithDraft = ScriptService.getScriptByPathWithDraft as unknown as ReturnType<typeof vi.fn>
const flowWithDraft = FlowService.getFlowByPathWithDraft as unknown as ReturnType<typeof vi.fn>
const flowLatestVersion = FlowService.getFlowLatestVersion as unknown as ReturnType<typeof vi.fn>
const appWithDraft = AppService.getAppByPathWithDraft as unknown as ReturnType<typeof vi.fn>

// A 404 from the generated client surfaces as a thrown ApiError-shaped object
// with `.status === 404`. The read path catches *only* this case.
function notFound() {
	return Object.assign(new Error('not found'), { status: 404 })
}

beforeEach(() => {
	vi.clearAllMocks()
})

describe('saveScriptDbDraft', () => {
	const baseScript = {
		path: 'u/test/foo',
		summary: 'Foo',
		description: '',
		content: 'def main(): pass',
		language: 'python3',
		schema: { type: 'object', properties: {} },
		kind: 'script',
		is_template: false
	} as any

	it('creates a draft_only anchor for a brand-new item, then upserts the draft row', async () => {
		await saveScriptDbDraft(ws, baseScript.path, baseScript, { itemExists: false })

		expect(scriptCreate).toHaveBeenCalledTimes(1)
		const anchor = scriptCreate.mock.calls[0][0]
		expect(anchor.workspace).toBe(ws)
		expect(anchor.requestBody.draft_only).toBe(true)
		expect(anchor.requestBody.path).toBe(baseScript.path)
		expect(anchor.requestBody.content).toBe(baseScript.content)
		expect(anchor.requestBody.language).toBe(baseScript.language)

		expect(draftCreate).toHaveBeenCalledTimes(1)
		const draftCall = draftCreate.mock.calls[0][0]
		expect(draftCall.workspace).toBe(ws)
		expect(draftCall.requestBody.typ).toBe('script')
		expect(draftCall.requestBody.path).toBe(baseScript.path)
		expect(draftCall.requestBody.value.content).toBe(baseScript.content)
		expect(draftCall.requestBody.value.draft_triggers).toEqual([])
	})

	it('skips the anchor when the item already exists (deployed or draft_only)', async () => {
		await saveScriptDbDraft(ws, baseScript.path, baseScript, { itemExists: true })
		expect(scriptCreate).not.toHaveBeenCalled()
		expect(draftCreate).toHaveBeenCalledTimes(1)
	})

	it('keeps draft_only/no-triggers/no-parent_hash on the anchor while preserving triggers in the draft row', async () => {
		await saveScriptDbDraft(
			ws,
			baseScript.path,
			{ ...baseScript, parent_hash: 'abc', draft_triggers: [{ x: 1 }] },
			{ itemExists: false }
		)
		const anchor = scriptCreate.mock.calls[0][0].requestBody
		expect(anchor.draft_only).toBe(true)
		expect(anchor.parent_hash).toBeUndefined()
		expect('draft_triggers' in anchor).toBe(false)

		const draftValue = draftCreate.mock.calls[0][0].requestBody.value
		expect(draftValue.draft_triggers).toEqual([{ x: 1 }])
	})
})

describe('saveFlowDbDraft', () => {
	const baseFlow = {
		path: 'u/test/flow',
		summary: 'My flow',
		description: 'desc',
		value: { modules: [] },
		schema: { type: 'object', properties: {} }
	} as any

	it('creates a draft_only anchor for a new flow, then upserts the draft row with path + empty triggers', async () => {
		await saveFlowDbDraft(ws, baseFlow.path, baseFlow, { itemExists: false })

		expect(flowCreate).toHaveBeenCalledTimes(1)
		const anchor = flowCreate.mock.calls[0][0].requestBody
		expect(anchor.draft_only).toBe(true)
		expect(anchor.path).toBe(baseFlow.path)
		expect(anchor.summary).toBe(baseFlow.summary)
		expect(anchor.value).toEqual(baseFlow.value)

		const draftCall = draftCreate.mock.calls[0][0].requestBody
		expect(draftCall.typ).toBe('flow')
		expect(draftCall.path).toBe(baseFlow.path)
		expect(draftCall.value.path).toBe(baseFlow.path)
		expect(draftCall.value.value).toEqual(baseFlow.value)
		expect(draftCall.value.draft_triggers).toEqual([])
	})

	it('skips the anchor when the flow already exists', async () => {
		await saveFlowDbDraft(ws, baseFlow.path, baseFlow, { itemExists: true })
		expect(flowCreate).not.toHaveBeenCalled()
		expect(draftCreate).toHaveBeenCalledTimes(1)
	})
})

describe('saveAppDbDraft', () => {
	const baseApp = {
		summary: 'My app',
		files: { '/main.tsx': 'export default 1' },
		runnables: { a: { type: 'inline' } },
		data: { tables: [] },
		policy: { execution_mode: 'publisher' },
		custom_path: 'my-app'
	} as any
	const path = 'u/test/app'

	it('bundles + creates a draft_only raw-app anchor for a new app, then upserts the nested draft value', async () => {
		const bundle = vi.fn(async () => ({ js: 'JS', css: 'CSS' }))
		await saveAppDbDraft(ws, path, baseApp, { itemExists: false, bundle })

		expect(bundle).toHaveBeenCalledTimes(1)
		expect(appCreate).toHaveBeenCalledTimes(1)
		const fd = appCreate.mock.calls[0][0].formData
		expect(fd.js).toBe('JS')
		expect(fd.css).toBe('CSS')
		expect(fd.app.draft_only).toBe(true)
		expect(fd.app.path).toBe(path)
		expect(fd.app.policy).toEqual(baseApp.policy)
		// anchor app value is the raw { files, runnables, data } — not the flattened draft value
		expect(fd.app.value).toEqual({
			files: baseApp.files,
			runnables: baseApp.runnables,
			data: baseApp.data
		})

		const draftCall = draftCreate.mock.calls[0][0].requestBody
		expect(draftCall.typ).toBe('app')
		expect(draftCall.path).toBe(path)
		// DB draft value nests the raw app under `value`, like the raw-app editor stores it
		expect(draftCall.value.value).toEqual({
			files: baseApp.files,
			runnables: baseApp.runnables,
			data: baseApp.data
		})
		expect(draftCall.value.path).toBe(path)
		expect(draftCall.value.summary).toBe(baseApp.summary)
		expect(draftCall.value.policy).toEqual(baseApp.policy)
		expect(draftCall.value.custom_path).toBe(baseApp.custom_path)
	})

	it('skips the anchor (and the bundle) when the app already exists', async () => {
		const bundle = vi.fn(async () => ({ js: 'JS', css: 'CSS' }))
		await saveAppDbDraft(ws, path, baseApp, { itemExists: true, bundle })
		expect(bundle).not.toHaveBeenCalled()
		expect(appCreate).not.toHaveBeenCalled()
		expect(draftCreate).toHaveBeenCalledTimes(1)
	})

	it('requires a bundle to create a new anchor', async () => {
		await expect(saveAppDbDraft(ws, path, baseApp, { itemExists: false })).rejects.toThrow(
			/bundle/i
		)
	})

	it('requires a policy to create a new anchor', async () => {
		const bundle = vi.fn(async () => ({ js: 'JS', css: 'CSS' }))
		await expect(
			saveAppDbDraft(ws, path, { ...baseApp, policy: undefined }, { itemExists: false, bundle })
		).rejects.toThrow(/policy/i)
	})
})

describe('readScriptDbDraft', () => {
	const path = 'f/scripts/existing'

	function deployed(extra: Record<string, any> = {}) {
		return {
			path,
			hash: 'deployed-hash',
			summary: 'deployed summary',
			description: 'deployed description',
			content: 'deployed content',
			language: 'bun',
			kind: 'script',
			...extra
		}
	}

	it('returns the not-found result when no row exists at the path', async () => {
		scriptWithDraft.mockRejectedValueOnce(notFound())
		const res = await readScriptDbDraft(ws, path)
		expect(res).toEqual({
			itemExists: false,
			deployedExists: false,
			draftOnly: false,
			hasDbDraft: false,
			value: undefined,
			meta: {}
		})
	})

	it('rethrows non-404 errors instead of swallowing them', async () => {
		scriptWithDraft.mockRejectedValueOnce(Object.assign(new Error('boom'), { status: 500 }))
		await expect(readScriptDbDraft(ws, path)).rejects.toThrow('boom')
	})

	it('returns the deployed value when the row has no draft', async () => {
		scriptWithDraft.mockResolvedValueOnce(deployed())
		const res = await readScriptDbDraft(ws, path)
		expect(res.itemExists).toBe(true)
		expect(res.deployedExists).toBe(true)
		expect(res.draftOnly).toBe(false)
		expect(res.hasDbDraft).toBe(false)
		expect(res.value?.content).toBe('deployed content')
		expect(res.meta).toEqual({ remoteRev: 'deployed-hash', remoteDraftRev: undefined })
	})

	it('returns the draft value (not the deployed one) when a draft row exists', async () => {
		scriptWithDraft.mockResolvedValueOnce(
			deployed({
				draft_created_at: '2026-05-22T10:00:00Z',
				draft: {
					path,
					summary: 'db draft summary',
					description: 'db draft description',
					content: 'draft content',
					language: 'bun',
					kind: 'script'
				}
			})
		)
		const res = await readScriptDbDraft(ws, path)
		expect(res.itemExists).toBe(true)
		expect(res.deployedExists).toBe(true)
		expect(res.draftOnly).toBe(false)
		expect(res.hasDbDraft).toBe(true)
		expect(res.value?.content).toBe('draft content')
		expect(res.value?.summary).toBe('db draft summary')
		expect(res.meta).toEqual({
			remoteRev: 'deployed-hash',
			remoteDraftRev: '2026-05-22T10:00:00Z'
		})
	})

	it('marks a draft_only row as not-deployed', async () => {
		scriptWithDraft.mockResolvedValueOnce(
			deployed({
				draft_only: true,
				draft_created_at: '2026-05-22T10:00:00Z',
				draft: { path, summary: 's', content: 'draft content', language: 'bun', kind: 'script' }
			})
		)
		const res = await readScriptDbDraft(ws, path)
		expect(res.itemExists).toBe(true)
		expect(res.deployedExists).toBe(false)
		expect(res.draftOnly).toBe(true)
		expect(res.hasDbDraft).toBe(true)
		expect(res.value?.content).toBe('draft content')
	})
})

describe('readFlowDbDraft', () => {
	const path = 'f/flows/existing'

	function deployed(extra: Record<string, any> = {}) {
		return {
			path,
			summary: 'deployed summary',
			description: 'deployed description',
			value: { modules: [] },
			schema: { properties: { deployed: { type: 'boolean' } } },
			edited_by: 'admin',
			edited_at: '2026-05-22T09:00:00Z',
			archived: false,
			extra_perms: {},
			...extra
		}
	}

	it('returns the not-found result when no row exists at the path', async () => {
		flowWithDraft.mockRejectedValueOnce(notFound())
		const res = await readFlowDbDraft(ws, path)
		expect(res).toEqual({
			itemExists: false,
			deployedExists: false,
			draftOnly: false,
			hasDbDraft: false,
			value: undefined,
			meta: {}
		})
		// No remoteRev lookup when the item doesn't exist.
		expect(flowLatestVersion).not.toHaveBeenCalled()
	})

	it('rethrows non-404 errors instead of swallowing them', async () => {
		flowWithDraft.mockRejectedValueOnce(Object.assign(new Error('boom'), { status: 500 }))
		await expect(readFlowDbDraft(ws, path)).rejects.toThrow('boom')
	})

	it('returns the deployed value with remoteRev from getFlowLatestVersion', async () => {
		flowWithDraft.mockResolvedValueOnce(deployed())
		flowLatestVersion.mockResolvedValueOnce({ id: 42 })
		const res = await readFlowDbDraft(ws, path)
		expect(res.itemExists).toBe(true)
		expect(res.deployedExists).toBe(true)
		expect(res.draftOnly).toBe(false)
		expect(res.hasDbDraft).toBe(false)
		expect(res.value?.value).toEqual({ modules: [] })
		expect(res.meta).toEqual({ remoteRev: 42, remoteDraftRev: undefined })
	})

	it('returns the draft value when a draft row exists', async () => {
		flowWithDraft.mockResolvedValueOnce(
			deployed({
				draft_created_at: '2026-05-22T10:00:00Z',
				draft: {
					path,
					summary: 'db draft summary',
					value: { modules: [{ id: 'a' }] },
					schema: { properties: { draft: { type: 'string' } } },
					edited_by: 'admin',
					edited_at: '2026-05-22T09:30:00Z',
					archived: false,
					extra_perms: {}
				}
			})
		)
		flowLatestVersion.mockResolvedValueOnce({ id: 42 })
		const res = await readFlowDbDraft(ws, path)
		expect(res.hasDbDraft).toBe(true)
		expect(res.value?.value).toEqual({ modules: [{ id: 'a' }] })
		expect(res.value?.summary).toBe('db draft summary')
		expect(res.meta).toEqual({ remoteRev: 42, remoteDraftRev: '2026-05-22T10:00:00Z' })
	})

	it('leaves remoteRev undefined for a draft_only flow with no deployed version', async () => {
		flowWithDraft.mockResolvedValueOnce(
			deployed({
				draft_only: true,
				draft_created_at: '2026-05-22T10:00:00Z',
				draft: {
					path,
					summary: 's',
					value: { modules: [] },
					edited_by: 'admin',
					edited_at: '2026-05-22T09:30:00Z',
					archived: false,
					extra_perms: {}
				}
			})
		)
		const res = await readFlowDbDraft(ws, path)
		expect(res.draftOnly).toBe(true)
		expect(res.deployedExists).toBe(false)
		expect(res.hasDbDraft).toBe(true)
		expect(res.meta.remoteRev).toBeUndefined()
		expect(res.meta.remoteDraftRev).toBe('2026-05-22T10:00:00Z')
		// A draft_only flow has no deployed version to look up.
		expect(flowLatestVersion).not.toHaveBeenCalled()
	})

	it('tolerates a 404 from getFlowLatestVersion (remoteRev undefined)', async () => {
		flowWithDraft.mockResolvedValueOnce(deployed())
		flowLatestVersion.mockRejectedValueOnce(notFound())
		const res = await readFlowDbDraft(ws, path)
		expect(res.deployedExists).toBe(true)
		expect(res.meta.remoteRev).toBeUndefined()
	})
})

describe('readAppDbDraft', () => {
	const path = 'f/apps/report'

	function deployed(extra: Record<string, any> = {}) {
		return {
			path,
			summary: 'deployed app',
			versions: [3, 4],
			value: {
				files: { '/src/App.tsx': 'deployed content' },
				runnables: {},
				data: { tables: [] }
			},
			policy: { execution_mode: 'publisher' },
			custom_path: 'report',
			...extra
		}
	}

	it('returns the not-found result when no row exists at the path', async () => {
		appWithDraft.mockRejectedValueOnce(notFound())
		const res = await readAppDbDraft(ws, path)
		expect(res).toEqual({
			itemExists: false,
			deployedExists: false,
			draftOnly: false,
			hasDbDraft: false,
			value: undefined,
			meta: {}
		})
	})

	it('rethrows non-404 errors instead of swallowing them', async () => {
		appWithDraft.mockRejectedValueOnce(Object.assign(new Error('boom'), { status: 500 }))
		await expect(readAppDbDraft(ws, path)).rejects.toThrow('boom')
	})

	it('returns the deployed value (shaped via appSourceToDraftValue) when no draft', async () => {
		appWithDraft.mockResolvedValueOnce(deployed())
		const res = await readAppDbDraft(ws, path)
		expect(res.itemExists).toBe(true)
		expect(res.deployedExists).toBe(true)
		expect(res.hasDbDraft).toBe(false)
		expect(res.value?.files).toEqual({ '/src/App.tsx': 'deployed content' })
		expect(res.value?.policy).toEqual({ execution_mode: 'publisher' })
		expect(res.value?.custom_path).toBe('report')
		expect(res.meta).toEqual({ remoteRev: 4, remoteDraftRev: undefined })
	})

	it('returns the draft value when a draft row exists', async () => {
		appWithDraft.mockResolvedValueOnce(
			deployed({
				draft_created_at: '2026-05-22T10:30:00Z',
				draft: {
					summary: 'saved app draft',
					value: {
						files: { '/src/App.tsx': 'draft content' },
						runnables: { main: { type: 'inline' } },
						data: { tables: ['orders'], datatable: 'db', schema: 'public' }
					},
					policy: { execution_mode: 'anonymous' },
					custom_path: 'report'
				}
			})
		)
		const res = await readAppDbDraft(ws, path)
		expect(res.hasDbDraft).toBe(true)
		expect(res.value?.files).toEqual({ '/src/App.tsx': 'draft content' })
		expect(res.value?.summary).toBe('saved app draft')
		expect(res.value?.policy).toEqual({ execution_mode: 'anonymous' })
		expect(res.value?.data).toEqual({ tables: ['orders'], datatable: 'db', schema: 'public' })
		expect(res.meta).toEqual({ remoteRev: 4, remoteDraftRev: '2026-05-22T10:30:00Z' })
	})

	it('marks a draft_only app as not-deployed', async () => {
		appWithDraft.mockResolvedValueOnce(
			deployed({
				draft_only: true,
				draft_created_at: '2026-05-22T10:30:00Z',
				draft: {
					summary: 's',
					value: { files: {}, runnables: {}, data: { tables: [] } },
					policy: { execution_mode: 'publisher' }
				}
			})
		)
		const res = await readAppDbDraft(ws, path)
		expect(res.draftOnly).toBe(true)
		expect(res.deployedExists).toBe(false)
		expect(res.hasDbDraft).toBe(true)
	})
})
