import { describe, expect, it, vi } from 'vitest'

// The toolset pulls in the script/flow editor tools, hence monaco. Same stand-ins as
// global/core.test.ts; nothing here executes a tool, so bare shapes are enough.
vi.mock('monaco-editor', () => ({
	editor: {},
	languages: {},
	KeyCode: {},
	Uri: { parse: (value: string) => ({ toString: () => value }) },
	MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 }
}))
vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features', () => ({
	getTypeScriptWorker: async () => async () => ({}),
	typescriptVersion: 'test'
}))
vi.mock('@codingame/monaco-vscode-languages-service-override', () => ({ default: () => ({}) }))
vi.mock('$lib/components/vscode', () => ({}))

// `globalToolsFor` withholds take_screenshot off Blink, and node reports a `Node.js/<v>`
// user agent — so without this the description sweep below silently skips a tool that
// ships in every real Chromium session. Claim the maximal toolset instead.
vi.stubGlobal('navigator', { userAgent: 'Mozilla/5.0 Chrome/120.0.0.0' })

import { globalTools, prepareGlobalSystemMessage, type SessionPromptContext } from './core'
import { appendPlanModeInstructions } from '../planMode'
import { PlanModeController } from '../planModeController.svelte'
import { pipelineTools } from '../pipeline/core'
import { assembleGlobalSystemMessage, assembleGlobalTools } from './globalAssembly'
import { filterSessionTools, sessionToolAllowed } from './sessionToolset'
import { capabilitiesForRole, fullSessionAccess } from './sessionAccess'
import type { SessionAccess, SessionCapability, SessionTool } from '../sessionCapabilities'

const ASSEMBLY_OPTS = {
	previewTools: true,
	pipelineContext: { folder: 'my_pipeline', mode: 'edit', nodes: [], assets: [] },
	mcpServers: [{ path: 'f/test/server' } as any]
}

/** Every tool that can reach a session toolset: what assembly builds, so a source added
 * there needs no edit here, unioned with the static sources so the set cannot shrink if
 * assembly stops reaching one, plus the plan tools the manager appends. */
function sessionReachableTools(): SessionTool<any>[] {
	const plan = new PlanModeController({ available: true } as any).availableTools
	const byName = new Map<string, SessionTool<any>>()
	for (const t of [
		...assembleGlobalTools(ASSEMBLY_OPTS),
		...globalTools,
		...pipelineTools,
		...plan
	]) {
		byName.set(t.def.function.name, t)
	}
	return [...byName.values()]
}

function withheldNames(access: SessionAccess): string[] {
	return sessionReachableTools()
		.filter((t) => !sessionToolAllowed(t, access))
		.map((t) => t.def.function.name)
}

/** The tools a session actually ships, through the same assembly production uses. */
function shippedSessionTools(access: SessionAccess) {
	return filterSessionTools(assembleGlobalTools(ASSEMBLY_OPTS), access)
}

function accessWith(capabilities: SessionCapability[]): SessionAccess {
	return new Set(capabilities)
}

/** The profiles a real session can hold, keyed for the test name, over every combination
 * of the role facts `capabilitiesForRole` reads. */
const REACHABLE_PROFILES = [
	...new Map(
		[false, true].flatMap((isAdmin) =>
			[false, true].flatMap((operator) =>
				// `checkDeployRules` bypasses on admin, so an admin has only the one outcome.
				(isAdmin ? [true] : [false, true]).map((deployRulesPass) => {
					const access = capabilitiesForRole({ isAdmin, operator, deployRulesPass })
					return [[...access].sort().join(',') || 'none', access] as const
				})
			)
		)
	).entries()
]

/** One per branch of getSessionContextPromptSection — each words the deploy target
 * differently, so a gate fixed in one branch can still leak in another. */
const SESSION_CONTEXTS: SessionPromptContext[] = [
	{ pendingForkOf: 'parent' },
	{ workspaceId: 'dev', parentWorkspaceId: 'parent', isDevWorkspace: true },
	{ workspaceId: 'fork', parentWorkspaceId: 'parent' },
	{ workspaceId: 'fork', forkParentUnknown: true },
	{ workspaceId: 'live' },
	{}
]

describe('session tool policies', () => {
	// Full access must be a no-op, or every existing session (and the ai_evals
	// baseline measured against it) changes behaviour.
	it('withholds nothing from a session with every capability', () => {
		const tools = sessionReachableTools()
		expect(filterSessionTools(tools, fullSessionAccess())).toHaveLength(tools.length)
	})

	it('withholds a tool that arrives without a policy', () => {
		expect(sessionToolAllowed({ def: globalTools[0].def }, fullSessionAccess())).toBe(false)
	})

	it('passes the toolset through untouched when access is unresolved', () => {
		const tools = globalTools.map((t) => ({ def: t.def }))
		expect(filterSessionTools(tools, undefined)).toHaveLength(tools.length)
	})

	// The kind is an argument, so its enum carries the permission, per the handler each kind
	// reaches. Asserted on the shipped schema, since that is all the model sees.
	it('offers the kind-taking tools only the kinds their handlers accept', () => {
		const shipped = (name: string, access: SessionAccess) =>
			shippedSessionTools(access).find((t) => t.def.function.name === name)!
		const kindsOf = (name: string, access: SessionAccess) =>
			(shipped(name, access).def.function.parameters as any).properties.type.enum

		// A developer a protection rule refuses keeps the two kinds no rule reaches.
		const refused = accessWith(['write_draft', 'run_preview', 'manage_code'])
		expect(kindsOf('deploy_workspace_item', refused)).toEqual(['schedule', 'trigger'])
		expect(kindsOf('delete_workspace_item', refused)).toEqual(['schedule', 'trigger'])

		// An operator where no rule applies: only the code handlers refuse them.
		const operator = accessWith(['deploy'])
		expect(kindsOf('deploy_workspace_item', operator)).toEqual([
			'schedule',
			'trigger',
			'resource',
			'variable'
		])

		// Deleting a script is admin-only, where deploying one is not.
		const developer = accessWith(['write_draft', 'run_preview', 'manage_code', 'deploy'])
		expect(kindsOf('deploy_workspace_item', developer)).toContain('script')
		expect(kindsOf('delete_workspace_item', developer)).not.toContain('script')

		// Full access narrows nothing and ships the shared object itself, so the tool defs —
		// part of every iteration's cached prefix — are byte for byte what they were.
		const original = globalTools.find((t) => t.def.function.name === 'deploy_workspace_item')!
		expect(shipped('deploy_workspace_item', fullSessionAccess())).toBe(original)
		// And narrowing never writes through to it: the def is shared by every session, so a
		// mutation here would strip one user's kinds from everyone else's schema.
		expect((original.def.function.parameters as any).properties.type.enum).toContain('script')
	})

	// Naming a tool the model was not given produces invented calls. Asserted over the
	// ASSEMBLED message: the session-state, pipeline and plan-mode sections are each
	// appended by a different caller, and each can name a withheld tool.
	it.each(REACHABLE_PROFILES)(
		'never names a withheld tool in the assembled prompt (%s)',
		(_label, access) => {
			const withheld = withheldNames(access)
			// The tool DEFINITIONS ship alongside the prompt, so a withheld name in a
			// description is the same broken promise as one in the prompt.
			const defs = JSON.stringify(shippedSessionTools(access).map((t) => t.def))
			expect(withheld.filter((n) => defs.includes(n))).toEqual([])
			for (const previewTools of [false, true]) {
				for (const ctx of SESSION_CONTEXTS) {
					const msg = assembleGlobalSystemMessage(undefined, {
						previewTools,
						user: { username: 'alex', folders: ['shared'], folders_read: ['shared'] },
						access,
						sessionContext: ctx,
						pipelineContext: { folder: 'my_pipeline', mode: 'edit', nodes: [], assets: [] }
					})
					// Both decoration variants: the escalation one adds its own tool mentions.
					for (const blocks of [0, 9]) {
						const full = appendPlanModeInstructions(msg, blocks).content as string
						expect(withheld.filter((n) => full.includes(n))).toEqual([])
					}
				}
			}
		}
	)

	// A tool result is the one place the sweep above cannot see, and `get_instructions`
	// ships to every profile with guidance written around the draft tools by name.
	it('does not hand authoring guidance naming withheld tools to a session that cannot draft', async () => {
		const tool = globalTools.find((t) => t.def.function.name === 'get_instructions')!
		const call = (access: SessionAccess) =>
			tool.fn({
				args: { subject: 'script', language: 'bun' },
				workspace: 'ws',
				helpers: { access },
				toolId: 't1',
				toolCallbacks: { setToolStatus: () => {} }
			} as any) as Promise<string>

		const readOnly = accessWith([])
		const withheld = withheldNames(readOnly)
		const restricted = await call(readOnly)
		expect(withheld.filter((n) => restricted.includes(n))).toEqual([])
		// The gate is the whole test, so pin that it is not simply refusing everyone.
		expect(await call(fullSessionAccess())).toContain('write_script')
	})

	// A full-access profile must gate nothing at all: the text has to match the ungated
	// build byte for byte, or every session's cached prefix and the ai_evals baseline
	// move underneath us.
	it('builds an unchanged prompt when every capability is present', () => {
		const user = { username: 'alex', folders: ['shared'], folders_read: ['shared'] }
		for (const previewTools of [false, true]) {
			const ungated = prepareGlobalSystemMessage(undefined, { previewTools, user }).content
			const full = prepareGlobalSystemMessage(undefined, {
				previewTools,
				user,
				access: fullSessionAccess()
			}).content
			expect(full).toBe(ungated)
		}
	})
})
