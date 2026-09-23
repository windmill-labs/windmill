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
import { pipelineTools } from '../pipeline/core'
import { ENTER_PLAN_MODE_TOOL, EXIT_PLAN_MODE_TOOL } from '../planMode'
import { assembleGlobalSystemMessage, assembleGlobalTools } from './globalAssembly'
import { SESSION_TOOL_POLICIES, filterSessionTools, sessionToolAllowed } from './sessionToolset'
import {
	capabilitiesForRole,
	fullSessionAccess,
	type SessionAccess,
	type SessionCapability
} from './sessionAccess'

const ASSEMBLY_OPTS = {
	previewTools: true,
	pipelineContext: { folder: 'my_pipeline', mode: 'edit', nodes: [], assets: [] },
	mcpServers: [{ path: 'f/test/server' } as any]
}

/** Every tool name that can reach a session toolset: what assembly builds, so a source
 * added there needs no edit here, unioned with the static sources so the set cannot
 * shrink if assembly stops reaching one, plus the plan tools the manager appends. */
function assembledSessionToolNames(): string[] {
	return [
		...new Set([
			...assembleGlobalTools(ASSEMBLY_OPTS).map((t) => t.def.function.name),
			...globalTools.map((t) => t.def.function.name),
			...pipelineTools.map((t) => t.def.function.name),
			ENTER_PLAN_MODE_TOOL,
			EXIT_PLAN_MODE_TOOL
		])
	]
}

/** The tools a session actually ships, through the same assembly production uses. */
function shippedSessionTools(access: SessionAccess) {
	return filterSessionTools(assembleGlobalTools(ASSEMBLY_OPTS), access)
}

function accessWith(capabilities: SessionCapability[]): SessionAccess {
	return new Set(capabilities)
}

const profileKey = (access: SessionAccess) => [...access].sort().join(',')

/** The profiles a real session can hold, over every combination of the role facts
 * `capabilitiesForRole` reads. */
const REACHABLE_PROFILES = new Set(
	[false, true].flatMap((isAdmin) =>
		[false, true].flatMap((operator) =>
			// `checkDeployRules` bypasses on admin, so an admin has only the one outcome.
			(isAdmin ? [true] : [false, true]).map((deployRulesPass) =>
				profileKey(capabilitiesForRole({ isAdmin, operator, deployRulesPass }))
			)
		)
	)
)

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
	// The fail-closed guarantee: `sessionToolAllowed` withholds an unregistered tool,
	// so a tool shipped without a policy would silently vanish from restricted
	// sessions. This test is what turns that into a build failure instead.
	it('covers every tool that can reach a session toolset', () => {
		const missing = assembledSessionToolNames().filter((n) => !SESSION_TOOL_POLICIES[n])
		expect(missing).toEqual([])
	})

	it('does not carry policies for tools that no longer exist', () => {
		const assembled = new Set(assembledSessionToolNames())
		const stale = Object.keys(SESSION_TOOL_POLICIES).filter((n) => !assembled.has(n))
		expect(stale).toEqual([])
	})

	// Full access must be a no-op, or every existing session (and the ai_evals
	// baseline measured against it) changes behaviour.
	it('withholds nothing from a session with every capability', () => {
		const names = assembledSessionToolNames()
		const allowed = names.filter((n) => sessionToolAllowed(n, fullSessionAccess()))
		expect(allowed).toEqual(names)
	})

	it('passes the toolset through untouched when access is unresolved', () => {
		const tools = globalTools.map((t) => ({ def: t.def }))
		expect(filterSessionTools(tools, undefined)).toHaveLength(tools.length)
	})

	it('withholds draft writes, deploys and previews without the capability', () => {
		const readOnly = accessWith([])
		expect(sessionToolAllowed('write_script', readOnly)).toBe(false)
		expect(sessionToolAllowed('write_variable', readOnly)).toBe(false)
		expect(sessionToolAllowed('deploy_workspace_item', readOnly)).toBe(false)
		expect(sessionToolAllowed('delete_workspace_item', readOnly)).toBe(true)
		expect(sessionToolAllowed('test_run_script', readOnly)).toBe(false)
		expect(sessionToolAllowed('exec_datatable_sql', readOnly)).toBe(false)
		expect(sessionToolAllowed('list_workspace_items', readOnly)).toBe(true)
		expect(sessionToolAllowed('list_runs', readOnly)).toBe(true)
		expect(sessionToolAllowed('cancel_job', readOnly)).toBe(true)
	})

	// The authoring aids are reads the server serves anyone, so a session that cannot
	// author still gets them — withholding them would be stricter than the backend.
	it('keeps the authoring aids when drafts cannot be written', () => {
		const readOnly = accessWith([])
		expect(sessionToolAllowed('get_instructions', readOnly)).toBe(true)
		expect(sessionToolAllowed('search_npm_packages', readOnly)).toBe(true)
		expect(sessionToolAllowed('search_resource_types', readOnly)).toBe(true)
	})

	// Pinned because the name is what misleads: it sits among the authoring aids and reads
	// like one, but starts a job.
	it('gates get_db_schema on run_preview, like the other job-starting tools', () => {
		expect(sessionToolAllowed('get_db_schema', accessWith(['write_draft']))).toBe(false)
		expect(sessionToolAllowed('get_db_schema', accessWith(['run_preview']))).toBe(true)
	})

	// The backend runs create_folder through check_deploy_rules, so drafting alone is
	// not enough to make it usable.
	it('withholds create_folder from a session that cannot deploy', () => {
		expect(sessionToolAllowed('create_folder', accessWith(['write_draft', 'run_preview']))).toBe(
			false
		)
		expect(sessionToolAllowed('create_folder', accessWith(['write_draft', 'deploy']))).toBe(true)
	})

	// Schedules and triggers reach no deploy rule, so a workspace that refuses this user's
	// deploys still accepts those two kinds. Gating the kind-taking tools on `deploy` would
	// withhold operations the server performs.
	it('keeps the deploy tools when only the gated kinds are refused', () => {
		const noDeploy = accessWith(['write_draft', 'run_preview'])
		expect(sessionToolAllowed('deploy_workspace_item', noDeploy)).toBe(true)
		expect(sessionToolAllowed('delete_workspace_item', noDeploy)).toBe(true)
		expect(sessionToolAllowed('create_folder', noDeploy)).toBe(false)
	})

	// Drafts must stay cleanable after a role change.
	it('keeps discard_local_draft without write_draft, but not rebase_draft', () => {
		const readOnly = accessWith([])
		expect(sessionToolAllowed('discard_local_draft', readOnly)).toBe(true)
		expect(sessionToolAllowed('rebase_draft', readOnly)).toBe(false)
	})

	// The prompt is documentation OF the toolset, so it must never name a tool the same
	// profile withheld — an instruction to call a tool the model was not given is what
	// produces invented calls and promises the chat cannot keep.
	//
	// Asserted over the ASSEMBLED message, not `prepareGlobalSystemMessage` alone: what
	// actually ships is that plus the session-state section, the pipeline-editor section
	// and plan mode's decoration, each appended by a different caller, and gating only the
	// first looks correct while the others still name withheld tools. Both axes are swept —
	// every reachable profile, and every tool from the policy table — so neither a new tool
	// nor a new capability combination slips past.
	it.each([
		// The prompt is swept for every profile, reachable or not, since gating it costs
		// nothing. The tool DEFINITIONS are swept only for the reachable ones, because the
		// only way to satisfy the rest is to strip a sibling tool's name out of a
		// description that earns its place for real sessions.
		['read-only', []],
		['drafts, no deploy', ['write_draft', 'run_preview']],
		['drafts only', ['write_draft']],
		['drafts, no preview', ['write_draft', 'deploy']],
		['deploy, no drafts', ['deploy']]
	] as [string, SessionCapability[]][])(
		'never names a withheld tool in the assembled prompt (%s)',
		(_label, capabilities) => {
			const access = accessWith(capabilities)
			const reachable = REACHABLE_PROFILES.has(profileKey(access))
			const withheld = assembledSessionToolNames().filter((n) => !sessionToolAllowed(n, access))
			expect(withheld.length).toBeGreaterThan(0)
			// The tool DEFINITIONS ship alongside the prompt, so a withheld name in a
			// description is the same broken promise as one in the prompt.
			if (reachable) {
				const defs = JSON.stringify(shippedSessionTools(access).map((t) => t.def))
				expect(withheld.filter((n) => defs.includes(n))).toEqual([])
			}
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

	// The sweep above reads tool definitions and the prompt. A tool RESULT is the third
	// place a withheld name reaches the model, and the only one neither can see —
	// `get_instructions` ships to every profile and its guidance is written around the
	// draft tools by name.
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
		const withheld = assembledSessionToolNames().filter((n) => !sessionToolAllowed(n, readOnly))
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

	// Draft writes survive without `deploy`, and vice versa: the two are separate
	// backend gates (drafts.rs vs. the deploy protection rules), not one ladder.
	it('treats write_draft and deploy as independent', () => {
		const draftsOnly = accessWith(['write_draft'])
		expect(sessionToolAllowed('write_script', draftsOnly)).toBe(true)
		expect(sessionToolAllowed('create_folder', draftsOnly)).toBe(false)

		const deployOnly = accessWith(['deploy'])
		expect(sessionToolAllowed('write_script', deployOnly)).toBe(false)
		expect(sessionToolAllowed('create_folder', deployOnly)).toBe(true)
	})
})
