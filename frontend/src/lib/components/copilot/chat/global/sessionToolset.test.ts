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
import type { Tool } from '../shared'
import { everyChatTool } from '../groupingTools'
import { assembleGlobalSystemMessage, assembleGlobalTools } from './globalAssembly'
import { filterSessionTools, sessionToolAllowed } from './sessionToolset'
import { capabilitiesForRole, fullSessionAccess } from './sessionAccess'
import {
	canBuild,
	USE_MODE_ACCESS,
	type SessionAccess,
	type SessionCapability,
	type SessionTool
} from '../sessionCapabilities'
import { FULL_CAPABILITIES, legacyOperatorCapabilities } from '$lib/roleCapabilities'
import { buildsAnything } from '$lib/editRights'
import type { AuthorLevel, OperatorSettings, RoleCapabilities } from '$lib/gen'

const ASSEMBLY_OPTS = {
	previewTools: true,
	pipelineFolders: ['my_pipeline'],
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
	// Shipped, not merely allowed: a kind-taking tool left with no kind is dropped too.
	const shipped = new Set(
		filterSessionTools(sessionReachableTools(), access).map((t) => t.def.function.name)
	)
	return sessionReachableTools()
		.map((t) => t.def.function.name)
		.filter((n) => !shipped.has(n))
}

/** The tools a session actually ships, through the same assembly production uses. */
function shippedSessionTools(access: SessionAccess) {
	return filterSessionTools(assembleGlobalTools(ASSEMBLY_OPTS), access)
}

/** A session that can change the workspace, holding `capabilities` besides. */
function accessWith(capabilities: SessionCapability[]): SessionAccess {
	return new Set<SessionCapability>(['modify_workspace', ...capabilities])
}

const LEVELS: AuthorLevel[] = ['none', 'compose', 'full']

/** Every role a workspace can define, over the capabilities `capabilitiesForRole` reads: `full`
 * flows or apps require `code`, as `RoleCapabilities::validate` refuses otherwise. */
const ROLES: RoleCapabilities[] = [false, true]
	.flatMap((code) =>
		LEVELS.flatMap((flows) =>
			LEVELS.flatMap((apps) =>
				[false, true].flatMap((manage_schedules) =>
					[false, true].map((manage_triggers) => ({
						...FULL_CAPABILITIES,
						code,
						flows,
						apps,
						manage_schedules,
						manage_triggers
					}))
				)
			)
		)
	)
	.filter((c) => c.code || (c.flows !== 'full' && c.apps !== 'full'))

/** The profiles a real session can hold, keyed for the test name, over every combination
 * of the role facts `capabilitiesForRole` reads, and the read-only one. */
const REACHABLE_PROFILES: [string, SessionAccess][] = [
	...new Map(
		[false, true].flatMap((isAdmin) =>
			// `checkDeployRules` bypasses on admin, so an admin has only the one outcome.
			(isAdmin ? [true] : [false, true]).flatMap((deployRulesPass) =>
				ROLES.map((capabilities) => {
					const access = capabilitiesForRole({ isAdmin, deployRulesPass, capabilities })
					return [[...access].sort().join(',') || 'none', access] as const
				})
			)
		)
	).entries(),
	['read-only', USE_MODE_ACCESS]
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
		const filtered = filterSessionTools(tools, fullSessionAccess())
		expect(filtered).toHaveLength(tools.length)
		filtered.forEach((tool, i) => expect(tool).toBe(tools[i]))
	})

	// The mode picker reads the role and the toolset reads the access built from it: if they
	// disagree, the picker offers Build while the chat holds only Use's tools, or the reverse.
	it('agrees with the role on whether the session can build', () => {
		for (const capabilities of ROLES) {
			const access = capabilitiesForRole({ isAdmin: false, deployRulesPass: true, capabilities })
			expect(canBuild(access)).toBe(buildsAnything(capabilities))
		}
	})

	it('withholds a tool that arrives without a policy', () => {
		expect(sessionToolAllowed({ def: globalTools[0].def }, fullSessionAccess())).toBe(false)
	})

	it('passes the toolset through untouched when access is unresolved', () => {
		const tools = globalTools.map((t) => ({ def: t.def }))
		expect(filterSessionTools(tools, undefined)).toBe(tools)
	})

	// The kind is an argument, so its enum carries the permission, per the handler each kind
	// reaches. Asserted on the shipped schema, since that is all the model sees.
	it('offers the kind-taking tools only the kinds their handlers accept', () => {
		const shipped = (name: string, access: SessionAccess) =>
			shippedSessionTools(access).find((t) => t.def.function.name === name)!
		const kindsOf = (name: string, access: SessionAccess) =>
			(shipped(name, access).def.function.parameters as any).properties.type.enum

		const developerCaps: SessionCapability[] = [
			'write_draft',
			'write_flow_draft',
			'write_app_draft',
			'run_preview',
			'run_flow_preview',
			'manage_code',
			'manage_flows',
			'manage_apps',
			'manage_schedules',
			'manage_triggers'
		]

		// A developer a protection rule refuses keeps the two kinds no rule reaches.
		const refused = accessWith(developerCaps)
		expect(kindsOf('deploy_workspace_item', refused)).toEqual(['schedule', 'trigger'])
		expect(kindsOf('delete_workspace_item', refused)).toEqual(['schedule', 'trigger'])

		// An operator where no rule applies: only the code handlers refuse them.
		const operator = accessWith(['deploy', 'manage_schedules', 'manage_triggers'])
		expect(kindsOf('deploy_workspace_item', operator)).toEqual([
			'schedule',
			'trigger',
			'resource',
			'variable'
		])

		// A workspace that withdrew schedule writes from operators: `gate_operator_writes`.
		const locked = accessWith(['deploy', 'manage_triggers'])
		expect(kindsOf('delete_workspace_item', locked)).toEqual(['trigger', 'resource', 'variable'])

		// A builder operator deploys flows too, and still no script or app.
		const builder = accessWith(['deploy', 'manage_flows', 'manage_schedules', 'manage_triggers'])
		expect(kindsOf('deploy_workspace_item', builder)).toContain('flow')
		expect(kindsOf('deploy_workspace_item', builder)).not.toContain('script')
		expect(kindsOf('deploy_workspace_item', builder)).not.toContain('app')

		// An app builder gets the app kind alone, and rebases its own stale app drafts.
		const appBuilder = accessWith(['deploy', 'manage_apps', 'write_app_draft'])
		expect(kindsOf('deploy_workspace_item', appBuilder)).toContain('app')
		expect(kindsOf('deploy_workspace_item', appBuilder)).not.toContain('flow')
		expect(kindsOf('rebase_draft', appBuilder)).toEqual(['app'])
		// A session left with no kind must not be offered an empty enum.
		expect(shippedSessionTools(operator).map((t) => t.def.function.name)).not.toContain(
			'rebase_draft'
		)

		// Deleting a script is admin-only, where deploying one is not.
		const developer = accessWith([...developerCaps, 'deploy'])
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

	// Keyed on the tags, so a tool added untagged stays out of a production workspace.
	it('ships a read-only session the reads and the deployed runs, and nothing that writes', () => {
		const shipped = shippedSessionTools(USE_MODE_ACCESS)
		expect(shipped.filter((t) => !t.planModeSafe && !t.readOnlySafe)).toEqual([])
		const names = shipped.map((t) => t.def.function.name)
		expect(names).toEqual(expect.arrayContaining(['read_workspace_item', 'list_runs', 'run_flow']))
		for (const write of ['write_script', 'test_run_script', 'deploy_workspace_item']) {
			expect(names).not.toContain(write)
		}
	})

	it('offers the workspace settings tools to admins only', () => {
		const settingsTools = ['get_workspace_settings', 'set_workspace_settings']
		for (const [, access] of REACHABLE_PROFILES) {
			const names = shippedSessionTools(access).map((t) => t.def.function.name)
			const offered = settingsTools.filter((n) => names.includes(n))
			expect(offered).toEqual(access.has('admin') ? settingsTools : [])
		}
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
						pipelineFolders: ['my_pipeline']
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
		const call = (access: SessionAccess, subject = 'script') =>
			tool.fn({
				args: { subject, language: 'bun' },
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

		// A builder gets the guide for their kind, rewritten without the inline-code tools.
		for (const [capability, subject, used] of [
			['write_flow_draft', 'flow', 'patch_flow_json'],
			['write_app_draft', 'app', 'write_app_runnable']
		] as const) {
			const builder = accessWith(['deploy', capability])
			const guide = await call(builder, subject)
			expect(guide).toContain(used)
			// The shared reference below still teaches inline code, so the guide must say to skip it.
			expect(guide).toContain('skip what it says')
			expect(withheldNames(builder).filter((n) => guide.includes(n))).toEqual([])
		}
	})

	it('tells a composing role hub scripts are refused only when it lacks code', async () => {
		const tool = globalTools.find((t) => t.def.function.name === 'get_instructions')!
		const texts = async (code: boolean) => {
			const access = capabilitiesForRole({
				isAdmin: false,
				deployRulesPass: true,
				capabilities: { ...FULL_CAPABILITIES, code, flows: 'compose', apps: 'compose' }
			})
			const guide = (subject: string) =>
				tool.fn({
					args: { subject },
					workspace: 'ws',
					helpers: { access },
					toolId: 't1',
					toolCallbacks: { setToolStatus: () => {} }
				} as any) as Promise<string>
			return {
				flow: await guide('flow'),
				app: await guide('app'),
				agent: await guide('agent'),
				prompt: prepareGlobalSystemMessage(undefined, { user: { username: 'u' }, access })
					.content as string
			}
		}
		const refusesHub = /refuse[sd]? [^.]*hub|hub [^.]*(are|is) refused/i
		const without = await texts(false)
		const withCode = await texts(true)
		for (const subject of ['flow', 'app'] as const) {
			expect(without[subject]).toMatch(refusesHub)
			expect(withCode[subject]).not.toMatch(refusesHub)
		}
		expect(withCode.flow).toContain('deployed in the workspace or hub scripts')
		// Without `code` the agent guide names only deployed scripts, so it never mentions hub scripts.
		expect(without.agent).not.toContain('hub script')
		expect(withCode.agent).toContain('hub script')
		expect(without.prompt).toContain('the server refuses inline code and hub scripts')
		expect(withCode.prompt).not.toMatch(refusesHub)
	})

	// A data setup needs SQL and inline runnables, neither of which a builder has.
	it('offers a builder no init_app data setup', async () => {
		const tool = globalTools.find((t) => t.def.function.name === 'init_app')!
		const properties = async (access: SessionAccess) =>
			((await tool.schemaFor!({ access })).function.parameters as any).properties
		const builder = await properties(accessWith(['deploy', 'write_app_draft']))
		const full = await properties(fullSessionAccess())
		expect(builder.data).toBeUndefined()
		expect(full.data).toBeDefined()
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

	// The operator's own rights outrun the chat's tools: what they schedule in the UI must not
	// be sent to an admin.
	it("sends an operator to the UI for what their role allows and the chat's tools do not", () => {
		const prompt = (operatorSettings: Partial<OperatorSettings>): string =>
			prepareGlobalSystemMessage(undefined, {
				user: { username: 'op' },
				access: capabilitiesForRole({
					isAdmin: false,
					deployRulesPass: true,
					capabilities: legacyOperatorCapabilities(operatorSettings as OperatorSettings)
				})
			}).content as string

		// None of these is sent to the UI: the chat drafts them itself.
		expect(prompt({})).not.toContain('they can still edit themselves on their pages')
		expect(prompt({})).toContain(
			'lets them create and edit resources, variables, schedules, and triggers'
		)
		expect(prompt({})).toContain('schedule a deployed script or flow themselves')
		expect(prompt({ manage_schedules: false })).not.toContain('schedule a deployed script')
	})
})

describe('lookup tools', () => {
	// A lookup's row folds into a collapsed group, so it must be a call plan mode lets run; and
	// the transcript looks a name up across modes, so a name is a lookup everywhere or nowhere.
	it('are plan-mode-safe, and agree on every name across modes', () => {
		const tools = everyChatTool()
		const name = (t: Tool<any>) => t.def.function.name
		expect(tools.filter((t) => t.lookup && !t.planModeSafe).map(name)).toEqual([])
		const lookup = new Map<string, Set<boolean>>()
		for (const t of tools) lookup.set(name(t), (lookup.get(name(t)) ?? new Set()).add(!!t.lookup))
		expect([...lookup].filter(([, values]) => values.size > 1).map(([n]) => n)).toEqual([])
	})
})

describe('test-run tools', () => {
	// The transcript looks a name up across modes, so a name runs the same kind of item in every
	// mode that offers it, or is a test run in none.
	it('agree on every name across modes', () => {
		const kinds = new Map<string, Set<string | undefined>>()
		for (const t of everyChatTool()) {
			const name = t.def.function.name
			kinds.set(name, (kinds.get(name) ?? new Set()).add(t.testRun))
		}
		expect([...kinds].filter(([, values]) => values.size > 1).map(([n]) => n)).toEqual([])
	})
})

describe('draft write tools', () => {
	// Script, flow and app writes have a diff card or an edit group. Every other item kind's
	// writes need its own draft capability and show only the item's path, so a new one must say
	// how its rows fold.
	it('include every write needing a per-kind draft capability', () => {
		const ownCard = new Set(['write_draft', 'write_flow_draft', 'write_app_draft'])
		const perKind = (t: (typeof globalTools)[number]) =>
			t.requires?.some((c) => /^write_\w+_draft$/.test(c) && !ownCard.has(c))
		expect(
			globalTools.filter((t) => perKind(t) && !t.draftWrite).map((t) => t.def.function.name)
		).toEqual([])
	})
})
