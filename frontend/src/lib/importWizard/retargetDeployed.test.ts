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
	failingTriggerPath: undefined as string | undefined,
	deployedJs: 'COMPILED',
	deletedResources: [] as string[],
	updatedRawApps: [] as any[],
	updatedApps: [] as any[],
	updatedFlows: [] as any[],
	flows: [] as any[],
	updatedTriggers: [] as any[]
}))

vi.mock('$lib/gen', () => ({
	ScriptService: {
		listSearchScript: vi.fn(async () => state.scripts),
		getScriptByPath: vi.fn(),
		createScript: vi.fn()
	},
	FlowService: {
		listSearchFlow: vi.fn(async () => state.flows),
		getFlowByPath: vi.fn(async ({ path }: any) => state.flows.find((f: any) => f.path === path)),
		updateFlow: vi.fn(async (p: any) => state.updatedFlows.push(p))
	},
	AppService: {
		listSearchApp: vi.fn(async () => state.apps),
		getAppByPath: vi.fn(async ({ path }: any) => state.apps.find((a) => a.path === path)),
		getPublicSecretOfLatestVersionOfApp: vi.fn(async () => 'secret'),
		updateApp: vi.fn(async (p: any) => state.updatedApps.push(p)),
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
			update: vi.fn(async (_w: string, path: string, body: any) => {
				if (path === state.failingTriggerPath) throw new Error('the update was rejected')
				state.updatedTriggers.push({ path, body })
			})
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
		text: async () => (url.endsWith('.js') ? state.deployedJs : 'STYLES')
	}))
)

import { applyRetarget, seesWholeWorkspace } from './retargetDeployed'

const FROM = 'f/proj/smtp'
const TO = 'f/shared/company_smtp'

/** A deployed raw app: sources plus runnables, with the bundle stored out of the value. */
const rawApp = {
	path: 'f/proj/dash',
	raw_app: true,
	value: { files: { '/App.tsx': 'v1' }, runnables: { send: { fields: { smtp: `$res:${FROM}` } } } }
}

async function run() {
	return applyRetarget({
		workspace: 'w',
		folder: 'proj',
		from: FROM,
		to: TO,
		seesWholeWorkspace: true
	})
}

describe('applyRetarget', () => {
	beforeEach(() => {
		state.apps = [rawApp]
		state.scripts = []
		state.triggers = []
		state.deployedJs = 'COMPILED'
		state.scheduleListError = undefined
		state.failingTriggerPath = undefined
		state.deletedResources = []
		state.updatedRawApps = []
		state.updatedApps = []
		state.flows = []
		state.updatedFlows = []
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

	// The bundle is compiled from the sources, so a `$res:` a source spells out is baked into
	// it. `retargetProjectExport` rewrites that copy on import, while /bundle.js is still one
	// of `files` — sending the deployed bundle back untouched would undo exactly that.
	it("rewrites the deployed bundle's own tokens before sending it back", async () => {
		state.deployedJs = `const cfg = "$res:${FROM}"; export default cfg`
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(outcome.stubDeleted).toBe(true)
		const sent = state.updatedRawApps[0]
		expect(sent.formData.js).toContain(`$res:${TO}`)
		expect(sent.formData.js).not.toContain(`$res:${FROM}`)
	})

	// A path the bundle names any other way is one nothing here can move, so the app is left
	// alone and the stub it still reads has to survive.
	it('keeps the stub when the bundle names the resource outside a $res: token', async () => {
		state.deployedJs = `const cfg = await getResource("${FROM}")`
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(outcome.stubDeleted).toBe(false)
		expect(state.deletedResources).toEqual([])
		expect(state.updatedRawApps).toEqual([])
	})

	// The path written inside a source file is a reference too, and no rewriter reaches it.
	//
	it('keeps the stub when a source file names the resource in code', async () => {
		state.apps = [
			{
				path: 'f/proj/dash',
				raw_app: true,
				value: {
					files: { '/App.tsx': `const c = await getResource("${FROM}")` },
					runnables: {}
				}
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(outcome.stubDeleted).toBe(false)
		expect(state.deletedResources).toEqual([])
		expect(state.updatedRawApps).toEqual([])
	})

	// A trigger holds its resource as a bare path in its own column, not as a `$res:` token.
	it('finds a trigger that holds the resource as a bare path', async () => {
		state.apps = []
		state.triggers = [
			{
				path: 'f/proj/ingest',
				script_path: 'f/proj/run',
				postgres_resource_path: FROM,
				permissioned_as: 'u/service_account',
				enabled: true
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(state.updatedTriggers[0].body.postgres_resource_path).toBe(TO)
		// The trigger keeps its own path even though it was the string being remapped.
		expect(state.updatedTriggers[0].path).toBe('f/proj/ingest')
		// Pointing a trigger at a credential must not also start it: `enabled` is left out so
		// the backend keeps whatever the trigger is set to.
		expect(state.updatedTriggers[0].body).not.toHaveProperty('enabled')
		// Nor run it as whoever picked the credential: a trigger states its identity as
		// `permissioned_as`, which the backend keeps only when told to preserve it.
		expect(state.updatedTriggers[0].body.permissioned_as).toBe('u/service_account')
		expect(state.updatedTriggers[0].body.preserve_permissioned_as).toBe(true)
		expect(state.deletedResources).toEqual([FROM])
	})

	// The stub is what a reference this run did not move still resolves through, so a write
	// that fails partway must not take it: the moved items and the rest both keep working.
	it('keeps the stub when a write fails, and reports what had already moved', async () => {
		state.apps = []
		state.triggers = [
			{ path: 'f/proj/first', postgres_resource_path: FROM },
			{ path: 'f/proj/second', postgres_resource_path: FROM }
		]
		state.failingTriggerPath = 'f/proj/second'
		const outcome = await run()
		expect(outcome.error).toContain('the update was rejected')
		expect(outcome.rewritten.map((r) => r.path)).toEqual(['f/proj/first'])
		expect(outcome.stubDeleted).toBe(false)
		expect(state.deletedResources).toEqual([])
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

	// Both spellings at once. The token is moved so the item stops depending on the stub for
	// what it could, and the mention it spells out still keeps the stub alive.
	it('rewrites the token of an app that also names the resource in code, and keeps the stub', async () => {
		state.apps = [
			{
				path: 'f/proj/dash',
				raw_app: true,
				value: {
					files: { '/App.tsx': `// the credential lives at ${FROM}` },
					runnables: { send: { fields: { smtp: `$res:${FROM}` } } }
				}
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(outcome.rewritten.map((r) => r.path)).toEqual(['f/proj/dash'])
		expect(outcome.gaps.map((g) => g.path)).toEqual(['f/proj/dash'])
		expect(outcome.stubDeleted).toBe(false)
		expect(JSON.stringify(state.updatedRawApps[0].formData.app.value)).toContain(`$res:${TO}`)
	})

	// Scripts, flows and resources share a path namespace, and the map holds a resource path.
	// The import's rewriters remap a runnable's own `path` on an exact match, which here would
	// repoint the step at the credential.
	it("moves an app's tokens without repointing a runnable that shares the path", async () => {
		state.apps = [
			{
				path: 'f/proj/page',
				value: {
					grid: [{ data: { type: 'runnableByPath', runType: 'script', path: FROM } }],
					inline: `$res:${FROM}`
				}
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		const sent = state.updatedApps[0]
		expect(sent.requestBody.value.inline).toBe(`$res:${TO}`)
		expect(sent.requestBody.value.grid[0].data.path).toBe(FROM)
	})

	// A token moves wherever it sits in the value, and a step's own path is left alone.
	it('moves a flow token outside the fields the import rewriter reached', async () => {
		state.flows = [
			{
				path: 'f/proj/pipeline',
				value: {
					modules: [
						{ id: 'a', summary: `reads $res:${FROM}`, value: { type: 'script', path: FROM } }
					]
				}
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		const sent = state.updatedFlows[0]
		expect(sent.requestBody.value.modules[0].summary).toBe(`reads $res:${TO}`)
		expect(sent.requestBody.value.modules[0].value.path).toBe(FROM)
	})

	// The listings run as the caller and row-level security filters them inside the query, so
	// an item a member cannot read is invisible rather than counted. The stub has to outlive
	// a scan that cannot see the whole workspace.
	it('keeps the stub when the caller is not shown the whole workspace', async () => {
		const outcome = await applyRetarget({
			workspace: 'w',
			folder: 'proj',
			from: FROM,
			to: TO,
			seesWholeWorkspace: false
		})
		expect(outcome.error).toBeUndefined()
		expect(outcome.stubDeleted).toBe(false)
		expect(state.deletedResources).toEqual([])
		expect(outcome.gaps.map((g) => g.path)).toContain('This workspace')
	})

	// A handler field names a runnable, and the map holds a resource path. A script sharing
	// that path is not the reference being moved.
	it("moves a trigger's resource field without repointing its error handler", async () => {
		state.apps = []
		state.triggers = [
			{
				path: 'f/proj/ingest',
				script_path: 'f/proj/run',
				postgres_resource_path: FROM,
				on_failure: `script/${FROM}`,
				error_handler_path: FROM,
				permissioned_as: 'u/service_account'
			}
		]
		const outcome = await run()
		expect(outcome.error).toBeUndefined()
		expect(state.updatedTriggers[0].body.postgres_resource_path).toBe(TO)
		expect(state.updatedTriggers[0].body.on_failure).toBe(`script/${FROM}`)
		// The bare spelling too: a handler path is a path and never a `$res:` token.
		expect(state.updatedTriggers[0].body.error_handler_path).toBe(FROM)
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

describe('seesWholeWorkspace', () => {
	// The user record is per-workspace and survives a workspace change, so reading `is_admin`
	// without checking which workspace it describes answers for the wrong one — and a wrong
	// yes here is what lets an RLS-filtered scan clear the stub for deletion.
	it.each([
		['admin of this workspace', { workspace_id: 'w', is_admin: true }, false, true],
		['admin of another workspace', { workspace_id: 'other', is_admin: true }, false, false],
		['member of this workspace', { workspace_id: 'w', is_admin: false }, false, false],
		['superadmin, record stale', { workspace_id: 'other', is_admin: false }, true, true],
		['no user record', undefined, false, false]
	])('%s', (_label, user, isSuperadmin, expected) => {
		expect(seesWholeWorkspace(user as any, isSuperadmin, 'w')).toBe(expected)
	})
})
