import { beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * The safety property: a retarget that cannot rewrite every referrer writes nothing at all.
 * Deleting the stub while an item still points at it is what breaks the imported project,
 * and that item is exactly the one nobody looks at afterwards.
 */

const state = vi.hoisted(() => ({
	apps: [] as any[],
	triggers: [] as any[],
	deletedResources: [] as string[],
	updatedRawApps: [] as any[],
	updatedTriggers: [] as any[]
}))

vi.mock('$lib/gen', () => ({
	ScriptService: {
		listSearchScript: vi.fn(async () => []),
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
		schedule: { badge: 'Schedule', list: vi.fn(async () => []) },
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

import { applyRetarget } from './retargetDeployed'

const FROM = 'f/proj/smtp'
const TO = 'f/shared/company_smtp'

/** A deployed raw app: sources plus runnables, with the bundle already stripped out. */
function rawApp(sources: Record<string, string>) {
	return {
		path: 'f/proj/dash',
		value: { files: sources, runnables: { send: { fields: { smtp: `$res:${FROM}` } } } }
	}
}

const exportedFiles = {
	'f/proj/dash': { '/App.tsx': 'v1', '/bundle.js': 'COMPILED', '/bundle.css': 'STYLES' }
}

async function run(exported?: typeof exportedFiles) {
	return applyRetarget({
		workspace: 'w',
		folder: 'proj',
		from: FROM,
		to: TO,
		hasEeLicense: true,
		exportedAppFiles: exported
	})
}

describe('applyRetarget', () => {
	beforeEach(() => {
		state.apps = [rawApp({ '/App.tsx': 'v1' })]
		state.triggers = []
		state.deletedResources = []
		state.updatedRawApps = []
		state.updatedTriggers = []
	})

	it("re-uploads the export's bundle rather than rebuilding it, then drops the stub", async () => {
		const outcome = await run(exportedFiles)
		expect(outcome.error).toBeUndefined()
		expect(outcome.stubDeleted).toBe(true)
		expect(state.deletedResources).toEqual([FROM])
		const sent = state.updatedRawApps[0]
		expect(sent.formData.js).toBe('COMPILED')
		expect(sent.formData.css).toBe('STYLES')
		// Rewritten, and the bundle entries stay out of the value the way the import leaves them.
		expect(JSON.stringify(sent.formData.app.value)).toContain(`$res:${TO}`)
		expect(Object.keys(sent.formData.app.value.files)).toEqual(['/App.tsx'])
	})

	// The bundle in the export was built from the export's sources. Once the deployed sources
	// have moved on, re-uploading it would revert whatever was changed since the import.
	it('refuses when the app has been edited since the import, and writes nothing', async () => {
		state.apps = [rawApp({ '/App.tsx': 'edited since' })]
		const outcome = await run(exportedFiles)
		expect(outcome.error).toContain('edited since the import')
		expect(outcome.stubDeleted).toBe(false)
		expect(state.updatedRawApps).toEqual([])
		expect(state.deletedResources).toEqual([])
	})

	it('refuses when the export does not describe a raw app it has to rewrite', async () => {
		const outcome = await run({})
		expect(outcome.error).toContain('does not describe')
		expect(state.deletedResources).toEqual([])
	})

	// A trigger holds its resource as a bare path in its own column, not as a `$res:` token.
	// Matching only the token spelling left the trigger on a stub that was then deleted.
	it('finds a trigger that holds the resource as a bare path', async () => {
		state.apps = []
		state.triggers = [
			{ path: 'f/proj/ingest', script_path: 'f/proj/run', postgres_resource_path: FROM }
		]
		const outcome = await run(exportedFiles)
		expect(outcome.error).toBeUndefined()
		expect(state.updatedTriggers[0].body.postgres_resource_path).toBe(TO)
		// The trigger keeps its own path even though it was the string being remapped.
		expect(state.updatedTriggers[0].path).toBe('f/proj/ingest')
		expect(state.deletedResources).toEqual([FROM])
	})
})
