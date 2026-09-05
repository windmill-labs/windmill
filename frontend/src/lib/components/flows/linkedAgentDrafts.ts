import { ResourceService, type FlowModule, type FlowValue, type InputTransform } from '$lib/gen'
import { UserDraft } from '$lib/userDraft.svelte'
import { canWrite } from '$lib/utils'
import type { UserExt } from '$lib/stores'
import { dfs } from './dfs'
import { flowLocalInputs, type AIAgentConfig } from './agentResourceUtils'
import type { AgentResourceState } from './agentDraft.svelte'
import type { AgentTool } from './agentToolUtils'

/** A step names its agent bare or as `$res:<path>`/`res://<path>`; all three are the same agent,
 *  and a draft index has to answer for a lookup written any of those ways. Same normalization as
 *  `linkedAgentToolsStore`, and as the `trim_start_matches` the worker applies. */
export function normalizeAgentRef(agentRef: string): string {
	return agentRef.replace(/^\$res:/, '').replace(/^res:\/\//, '')
}

/** Every `ai_agent` resource this flow links to, deduped. `dfs` walks agent tool nodes as well as
 *  branches and loops, so a nested linked agent tool is included. */
export function linkedAgentPaths(value: FlowValue | undefined): string[] {
	if (!value?.modules) return []
	const paths = new Set<string>()
	for (const module of dfs(value.modules, (m) => m)) {
		const v = module?.value as { type?: string; agent?: string } | undefined
		if (v?.type === 'aiagent' && v.agent) {
			paths.add(normalizeAgentRef(v.agent))
		}
	}
	return [...paths]
}

/**
 * The unsaved draft for an agent, freshest first: the cell an open agent editor is writing, then
 * what a `get_draft` response carried.
 *
 * Order matters. `UserDraftDbSyncer` debounces autosave by 1.5s (up to 10s), so the persisted row
 * lags an open editor by seconds — a test right after typing would otherwise run a stale prompt.
 */
export function agentDraftState(
	response: { draft?: unknown },
	path: string,
	workspace: string | undefined
): AgentResourceState | undefined {
	const live = UserDraft.get<AgentResourceState>('resource', path, { workspace })
	return live ?? (response.draft as AgentResourceState | undefined)
}

/** One linked agent whose resource the user has an unsaved draft for. */
export interface LinkedAgentDraft {
	path: string
	/** The draft's resource value: what a run of this agent would use. */
	args: AIAgentConfig
	/** The whole draft row, as the resource editors write it — the deploy payload. */
	state: AgentResourceState
	/** No deployed row at this path, so deploying has to create rather than update. */
	noDeployed: boolean
	/** Of the deployed resource, for `agentDraftCanWrite`. */
	extraPerms: Record<string, boolean>
}

/** Whether `user` may write this agent's resource. Split from the load so that resolving the
 *  drafts of a whole flow costs no `whoami` — only the deploy dialog needs the answer, and it
 *  looks the user up once for every agent it lists. */
export function agentDraftCanWrite(draft: LinkedAgentDraft, user: UserExt | undefined): boolean {
	return canWrite(draft.path, draft.extraPerms, user)
}

/**
 * The unsaved draft of every given `ai_agent` path, for the paths that have one. A path that fails
 * to load is reported as having no draft rather than throwing: a broken or unreadable link must not
 * block the test or the deploy that asked.
 */
export async function loadLinkedAgentDrafts(
	paths: string[],
	workspace: string | undefined
): Promise<Map<string, LinkedAgentDraft>> {
	const out = new Map<string, LinkedAgentDraft>()
	if (!workspace || paths.length === 0) return out
	await Promise.all(
		paths.map(async (path) => {
			try {
				const r = await ResourceService.getResource({ workspace, path, getDraft: true })
				const state = agentDraftState(r, path, workspace)
				if (!state) return
				out.set(path, {
					path,
					args: (state.args ?? {}) as AIAgentConfig,
					state,
					noDeployed: Boolean((r as { no_deployed?: boolean }).no_deployed),
					extraPerms: r.extra_perms ?? {}
				})
			} catch {
				// No draft we can act on.
			}
		})
	)
	return out
}

/**
 * Every argument a saved agent carries, as a static input transform. Not only the keys the agent
 * form renders: a run reads them all, and an agent holding its own `user_message` answers with it
 * when nothing overrides it. `tools` is the step's own roster rather than an input, so it rides on
 * the module's `tools` key instead.
 */
export function agentArgsToTransforms(args: AIAgentConfig): Record<string, InputTransform> {
	const it: Record<string, InputTransform> = {}
	for (const [key, value] of Object.entries(args ?? {})) {
		if (key === 'tools' || value === undefined) continue
		it[key] = { type: 'static', value } as InputTransform
	}
	return it
}

type AiAgentValue = Extract<FlowModule['value'], { type: 'aiagent' }>

/**
 * The standalone step a linked step's draft would run as: the draft's brain and tools inlined, with
 * the step's own flow-local inputs kept on top.
 *
 * The overlay order is the worker's (`ai_executor.rs`): its linked branch interpolates the whole
 * resource brain and only then writes `user_message`/`user_attachments` back from the step's own
 * args. `tool_inputs` stays untouched — the worker overlays it onto the tools in both branches, so
 * an inlined step keeps the host flow's tool bindings.
 */
export function inlineAgentDraft(value: AiAgentValue, args: AIAgentConfig): AiAgentValue {
	const { agent: _agent, ...rest } = value
	return {
		...rest,
		tools: (args.tools ?? []) as AgentTool[],
		input_transforms: {
			...agentArgsToTransforms(args),
			...flowLocalInputs(value.input_transforms as Record<string, InputTransform>)
		}
	} as AiAgentValue
}

/**
 * Replace every linked agent step that has a draft with the draft's own configuration, so a preview
 * runs what the agent editor is showing rather than the deployed resource. Returns a new value: the
 * flow editor hands its live store object to previews.
 */
export function inlineAgentDrafts(
	value: FlowValue,
	drafts: Map<string, LinkedAgentDraft>
): FlowValue {
	if (drafts.size === 0) return value
	// JSON rather than `structuredClone`: the flow editor's value is a Svelte `$state` proxy, which
	// `structuredClone` refuses outright. A flow value is JSON by definition — it is about to be
	// posted as one — so the round trip loses nothing this preview would have carried.
	const next = JSON.parse(JSON.stringify(value)) as FlowValue
	for (const module of dfs(next.modules ?? [], (m) => m)) {
		const v = module?.value as AiAgentValue | undefined
		if (v?.type !== 'aiagent' || !v.agent) continue
		const draft = drafts.get(normalizeAgentRef(v.agent))
		if (!draft) continue
		module.value = inlineAgentDraft(v, draft.args)
	}
	return next
}

/** Load the drafts this flow's linked agents have and inline them. The whole substitution, for a
 *  caller holding nothing but the value it is about to preview. */
export async function withAgentDrafts(
	value: FlowValue,
	workspace: string | undefined
): Promise<FlowValue> {
	const paths = linkedAgentPaths(value)
	if (paths.length === 0) return value
	return inlineAgentDrafts(value, await loadLinkedAgentDrafts(paths, workspace))
}
