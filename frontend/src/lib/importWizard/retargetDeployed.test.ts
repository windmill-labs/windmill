import { beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * The safety property: the stub survives anything the scan cannot account for. Rewriting an
 * item onto the chosen resource is safe on its own, so it always happens; deleting the stub
 * while an item still reads it is what breaks the imported project, and that item is exactly
 * the one nobody looks at afterwards.
 */

const state = vi.hoisted(() => ({
	apps: [] as any[],
	scripts: [] as any[],
	triggers: [] as any[],
	scheduleListError: undefined as any,
	deletedResources: [] as string[],
	updatedRawApps: [] as any[],
	updatedTriggers: [] as any[]
}))

vi.mock('$lib/gen', () => ({
	ScriptService: {
		listSearchScript: vi.fn(async () => state.scripts),
		getScriptByPath: vi.fn(),
		createScript: vi.fn()
	},
	FlowService: {
		listSearchFlow: vi.fn(async () => []),
		getFlowByPath: vi.fn(),
		updateFlow: vi.fn()
	},
	AppService: {
		listSearchApp: vi.fn(async () => state.apps),
		getAppByPath: vi.fn(async ({ path }: any) => state.apps.find((a) => a.path === path)),
		getPublicSecretOfLatestVersionOfApp: vi.fn(async () => 'secret'),
		updateApp: vi.fn(),
		updateAppRaw: vi.fn(async (p: any) => state.updatedRawApps.push(p))
	},
	ScheduleService: { updateSchedule: vi.fn() },
	ResourceService: {
		deleteResource: vi.fn(async ({ path }: any) => state.deletedResources.push(path))
	}
}))

vi.mock('$lib/components/triggers/workspaceTriggersList', () => ({
	TRIGGER_KINDS: {
		schedule: {
			badge: 'Schedule',
			list: vi.fn(async () => {
				if (state.scheduleListError) throw state.scheduleListError
				return []
			})
		},
		postgres: {
			badge: 'Postgres',
			list: vi.fn(async () => state.triggers),
			update: vi.fn(async (_w: string, path: string, body: any) =>
				state.updatedTriggers.push({ path, body })
			)
		}
	},
	WORKSPACE_TRIGGER_KINDS: ['schedule', 'postgres'],
	createWorkspaceTriggerDisabled: vi.fn(),
	triggerHandlerRefs: () => []
}))

vi.mock('$lib/components/apps/editor/appPolicy', () => ({ updatePolicy: vi.fn(async () => ({})) }))
vi.mock('$lib/sharedUtils', () => ({ updateRawAppPolicy: vi.fn(async () => ({})) }))

// The deployed bundle is served by secret, not through the generated client.
vi.stubGlobal(
	'fetch',
	vi.fn(async (url: string) => ({
		ok: true,
		status: 200,
		text: async () => (url.endsWith('.js') ? 'COMPILED' : 'STYLES')
	}))
)

import { applyRetarget } from './retargetDeployed'

const FROM = 'f/proj/smtp'
const TO = 'f/shared/company_smtp'

/** A deployed raw app: sources plus runnables, with the bundle stored out of the value. */
const rawApp = {
	path: 'f/proj/dash',
	value: { files: { '/App.tsx': 'v1' }, runnables: { send: { fields: { smtp: `$res:${FROM}` } } } }
}

async function run() {
	return applyRetarget({
		workspace: 'w',
		folder: 'proj',
		from: FROM,
		to: TO,
		hasEeLicense: true
	})
}

describe('applyRetarget', () => {
	beforeEach(() => {
		state.apps = [rawApp]
		state.scripts = []
		state.triggers = []
		state.scheduleListError = undefined
		state.deletedResources = []
		state.updatedRawApps = []
		state.updatedTriggers = []
	})

	// `updateAppRaw` refuses without a bundle, and the browser cannot rebuild one. The bundle
	// that goes back is the deployed one, read back by secret.
	it('sends the deployed bundle back with the rewritten value, then drops the stub', async () => {
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(outcome.gaps).toEqual([])
		expect(outcome.stubDeleted).toBe(true)
		expect(state.deletedResources).toEqual([FROM])
		const sent = state.updatedRawApps[0]
		expect(sent.formData.js).toBe('COMPILED')
		expect(sent.formData.css).toBe('STYLES')
		// Rewritten, and the bundle entries stay out of the value the way the import leaves them.
		expect(JSON.stringify(sent.formData.app.value)).toContain(`$res:${TO}`)
		expect(Object.keys(sent.formData.app.value.files)).toEqual(['/App.tsx'])
	})

	// A trigger holds its resource as a bare path in its own column, not as a `$res:` token.
	// Matching only the token spelling left the trigger on a stub that was then deleted.
	it('finds a trigger that holds the resource as a bare path', async () => {
		state.apps = []
		state.triggers = [
			{ path: 'f/proj/ingest', script_path: 'f/proj/run', postgres_resource_path: FROM }
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(state.updatedTriggers[0].body.postgres_resource_path).toBe(TO)
		// The trigger keeps its own path even though it was the string being remapped.
		expect(state.updatedTriggers[0].path).toBe('f/proj/ingest')
		expect(state.deletedResources).toEqual([FROM])
	})

	// Most trigger kinds are cargo features an instance may not compile in, and their routes
	// then 404. Reading that as a failed listing keeps the stub on every stock build.
	it('drops the stub when a trigger kind is not compiled in, keeps it when one truly fails', async () => {
		state.scheduleListError = { status: 404 }
		expect((await run()).gaps).toEqual([])
		expect(state.deletedResources).toEqual([FROM])

		state.deletedResources = []
		state.scheduleListError = { status: 500 }
		expect((await run()).gaps.map((g) => g.path)).toContain('Schedule triggers')
		expect(state.deletedResources).toEqual([])
	})

	// The referrer scan matches a bare path anywhere in an app's value, while the rewriter only
	// relocates `$res:` tokens and runnable paths. Rewriting such an app would claim a move that
	// did not happen, and the stub it still reads would go.
	it('leaves an app holding the resource path as a bare string alone, and keeps the stub', async () => {
		state.apps = [
			{
				path: 'f/proj/page',
				value: { grid: [{ data: { input: { type: 'static', value: FROM } } }] }
			}
		]
		const outcome = await run()
		expect(outcome.rewritten).toEqual([])
		expect(outcome.gaps.map((g) => g.path)).toEqual(['f/proj/page'])
		expect(state.deletedResources).toEqual([])
	})

	// `listSearchApp` caps at 1000 rows server-side, unordered and unpaginated, so a full page
	// may not hold the project's own app — and the stub would go anyway.
	it('keeps the stub when the app listing comes back at its server-side cap', async () => {
		state.apps = Array.from({ length: 1000 }, (_, i) => ({ path: `f/other/a${i}`, value: {} }))
		const outcome = await run()
		expect(outcome.gaps.map((g) => g.path)).toContain('Apps')
		expect(state.deletedResources).toEqual([])
	})

	// The listings are workspace-wide. An item outside the project's folder is the user's own,
	// so it is not rewritten — and it is exactly why the stub it reads has to stay.
	it('rewrites what it owns and keeps the stub for a reference outside the project', async () => {
		state.apps = []
		state.scripts = [{ path: 'u/alice/report', content: `$res:${FROM}` }]
		state.triggers = [
			{ path: 'f/proj/ingest', script_path: 'f/proj/run', postgres_resource_path: FROM }
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(state.updatedTriggers[0].body.postgres_resource_path).toBe(TO)
		expect(outcome.gaps.map((g) => g.path)).toEqual(['u/alice/report'])
		expect(outcome.stubDeleted).toBe(false)
		expect(state.deletedResources).toEqual([])
	})
})
