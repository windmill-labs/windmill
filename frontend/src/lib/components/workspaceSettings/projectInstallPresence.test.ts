import { beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * A retry must not re-write what the destination already has. The interesting kind is the
 * trigger: it is the one item the import creates through an API that rejects an existing
 * path outright, so replaying it turns "already there" into a reported failure.
 */

const created = vi.hoisted(() => ({ triggers: [] as string[] }))

vi.mock('$lib/gen', () => {
	const nothing = vi.fn(async () => [])
	return {
		AppService: { createApp: vi.fn(), listApps: nothing },
		FlowService: { createFlow: vi.fn(), listFlows: nothing },
		FolderService: { createFolder: vi.fn() },
		ResourceService: { createResource: vi.fn(), listResource: nothing },
		ScriptService: { createScript: vi.fn(), listScripts: nothing },
		VariableService: { createVariable: vi.fn(), listVariable: nothing },
		WorkspaceService: { listDataTables: vi.fn(async () => []) }
	}
})

vi.mock('../triggers/workspaceTriggersList', () => ({
	TRIGGER_KINDS: {
		schedule: { badge: 'Schedule', resourceField: undefined },
		http: { badge: 'HTTP', resourceField: undefined }
	},
	createWorkspaceTriggerDisabled: vi.fn(async (_ws: string, t: { path: string; kind: string }) => {
		created.triggers.push(`${t.kind}:${t.path}`)
	}),
	triggerHandlerRefs: () => []
}))

import { installProject, presenceKey } from './projectInstall'

const exportData = {
	project: { slug: 'calendly', name: 'Calendly', summary: '', readme: null },
	scripts: [],
	flows: [],
	apps: [],
	resources: [],
	variables: [],
	triggers: [
		{
			kind: 'schedule',
			path: 'f/calendly/nightly',
			runnable_path: 'f/calendly/sync',
			runnable_kind: 'script',
			summary: null,
			config: {}
		}
	],
	migrations: []
} as any

async function run(alreadyPresent?: Set<string>) {
	const results: any[] = []
	await installProject({
		workspace: 'w',
		exportData,
		folder: 'calendly',
		migrations: [],
		hasEeLicense: true,
		alreadyPresent,
		onResult: (r) => results.push(r)
	})
	return results
}

describe('installProject presence', () => {
	beforeEach(() => {
		created.triggers = []
	})

	it('creates a trigger the destination does not have', async () => {
		const results = await run(new Set())
		expect(created.triggers).toEqual(['schedule:f/calendly/nightly'])
		expect(results).toContainEqual({ path: 'f/calendly/nightly', ok: true })
	})

	// Without the skip the retry calls the create API again, which rejects the existing
	// path, and the row reads as a failure for something that is already there.
	it('skips a trigger that is already there instead of re-creating it', async () => {
		const results = await run(new Set([presenceKey('trigger:schedule', 'f/calendly/nightly')]))
		expect(created.triggers).toEqual([])
		expect(results).toContainEqual({ path: 'f/calendly/nightly', ok: true, skipped: true })
	})

	// Kinds share one `f/<folder>/` namespace, so the key has to carry the kind: a script of
	// the same name is not this trigger and must not stand in for it.
	it('does not let another kind at the same path mask the trigger', async () => {
		const results = await run(new Set([presenceKey('script', 'f/calendly/nightly')]))
		expect(created.triggers).toEqual(['schedule:f/calendly/nightly'])
		expect(results).toContainEqual({ path: 'f/calendly/nightly', ok: true })
	})

	// Each trigger kind is its own table keyed on (path, workspace_id), so a workspace can
	// hold a schedule and an HTTP trigger both called `f/calendly/nightly`. The one that
	// exists must not answer for the one that does not.
	it('does not let another trigger kind at the same path mask this one', async () => {
		const results = await run(new Set([presenceKey('trigger:http', 'f/calendly/nightly')]))
		expect(created.triggers).toEqual(['schedule:f/calendly/nightly'])
		expect(results).toContainEqual({ path: 'f/calendly/nightly', ok: true })
	})
})
