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

import {
	globalTools,
	getSessionContextPromptSection,
	prepareGlobalSystemMessage,
	type SessionPromptContext
} from './core'
import { appendPlanModeInstructions } from '../planMode'
import { pipelineTools } from '../pipeline/core'
import { createMcpTools } from './mcpTools'
import { ENTER_PLAN_MODE_TOOL, EXIT_PLAN_MODE_TOOL } from '../planMode'
import { SESSION_TOOL_POLICIES, filterSessionTools, sessionToolAllowed } from './sessionToolset'
import { fullSessionAccess, type SessionAccess, type SessionCapability } from './sessionAccess'

/** Every tool name that can reach a session's toolset. `globalTools` is only part of
 * it — pipeline and MCP tools are appended by `configureGlobalMode`, and plan mode's
 * at request time — which is exactly why the policy table is keyed by name rather
 * than declared on `globalTools`. */
function assembledSessionToolNames(): string[] {
	const mcp = createMcpTools([{ path: 'f/test/server' } as any])
	return [
		...globalTools.map((t) => t.def.function.name),
		...pipelineTools.map((t) => t.def.function.name),
		...mcp.map((t) => t.def.function.name),
		ENTER_PLAN_MODE_TOOL,
		EXIT_PLAN_MODE_TOOL
	]
}

function accessWith(capabilities: SessionCapability[]): SessionAccess {
	return { workspace: 'test', capabilities: new Set(capabilities) }
}

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
		const allowed = names.filter((n) => sessionToolAllowed(n, fullSessionAccess('test')))
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
		expect(sessionToolAllowed('test_run_script', readOnly)).toBe(false)
		expect(sessionToolAllowed('exec_datatable_sql', readOnly)).toBe(false)
		expect(sessionToolAllowed('list_workspace_items', readOnly)).toBe(true)
		expect(sessionToolAllowed('list_runs', readOnly)).toBe(true)
		expect(sessionToolAllowed('cancel_job', readOnly)).toBe(true)
	})

	// Relevance is the second axis: these need no capability, so `requires` alone
	// would keep advertising them to a session that can never author anything.
	it('drops authoring aids when drafts cannot be written', () => {
		const readOnly = accessWith(['deploy'])
		expect(sessionToolAllowed('get_instructions', readOnly)).toBe(false)
		expect(sessionToolAllowed('search_npm_packages', readOnly)).toBe(false)
		expect(sessionToolAllowed('create_folder', readOnly)).toBe(false)
		expect(sessionToolAllowed('search_docs', readOnly)).toBe(true)
	})

	// The prompt is documentation OF the toolset, so it must never name a tool the same
	// profile withheld — an instruction to call a tool the model was not given is what
	// produces invented calls and promises the chat cannot keep.
	//
	// Asserted over the ASSEMBLED message, not `prepareGlobalSystemMessage` alone: what
	// actually ships is that plus the session-state section and plan mode's decoration,
	// each appended by a different caller, and gating only the first looks correct while
	// the other two still name withheld tools. Both axes are swept — every reachable
	// profile, and every tool from the policy table — so neither a new tool nor a new
	// capability combination slips past.
	it.each([
		['read-only', []],
		['drafts, no deploy', ['write_draft', 'run_preview']],
		['drafts, no preview', ['write_draft', 'deploy']],
		['deploy, no drafts', ['deploy']]
	] as [string, SessionCapability[]][])(
		'never names a withheld tool in the assembled prompt (%s)',
		(_label, capabilities) => {
			const access = accessWith(capabilities)
			const withheld = assembledSessionToolNames().filter((n) => !sessionToolAllowed(n, access))
			expect(withheld.length).toBeGreaterThan(0)
			for (const previewTools of [false, true]) {
				for (const ctx of SESSION_CONTEXTS) {
					let msg = prepareGlobalSystemMessage(undefined, {
						previewTools,
						user: { username: 'alex', folders: ['shared'], folders_read: ['shared'] },
						access
					})
					msg = {
						...msg,
						content: (msg.content as string) + getSessionContextPromptSection(ctx, access)
					}
					// Both decoration variants: the escalation one adds its own tool mentions.
					for (const blocks of [0, 9]) {
						const full = appendPlanModeInstructions(msg, blocks).content as string
						expect(withheld.filter((n) => full.includes(n))).toEqual([])
					}
				}
			}
		}
	)

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
				access: fullSessionAccess('test')
			}).content
			expect(full).toBe(ungated)
		}
	})

	// Draft writes survive without `deploy`, and vice versa: the two are separate
	// backend gates (drafts.rs vs. the deploy protection rules), not one ladder.
	it('treats write_draft and deploy as independent', () => {
		const draftsOnly = accessWith(['write_draft'])
		expect(sessionToolAllowed('write_script', draftsOnly)).toBe(true)
		expect(sessionToolAllowed('deploy_workspace_item', draftsOnly)).toBe(false)

		const deployOnly = accessWith(['deploy'])
		expect(sessionToolAllowed('write_script', deployOnly)).toBe(false)
		expect(sessionToolAllowed('deploy_workspace_item', deployOnly)).toBe(true)
	})
})
