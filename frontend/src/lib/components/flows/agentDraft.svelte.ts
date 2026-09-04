import { untrack } from 'svelte'
import { get } from 'svelte/store'
import { deepEqual } from 'fast-equals'
import { ResourceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { canWrite } from '$lib/utils'
import { userStore } from '$lib/stores'
import { getUserExt } from '$lib/user'
import { useTriggerDraftSync, type TriggerDraftSync } from '../triggers/useTriggerDraftSync.svelte'
import { logReusableAgentUsage } from './agentTelemetry'
import {
	AGENT_BRAIN_LABELS,
	agentEditorRefusal,
	transformValuedBrainKeys,
	type AIAgentConfig
} from './agentResourceUtils'
// Lives here rather than beside the other config helpers: `agentResourceUtils` is a leaf, and
// importing tool-name validation into it would cycle back through `flowInfers` and pull the whole
// editor into every module that reads a brain key.
import { getToolNameError } from './agentToolUtils'

/**
 * Why a run of this config would fail before it started, if it would. Only the conditions the
 * worker itself rejects: a provider it cannot call, or a tool name it cannot put in the schema it
 * gives the model. Everything else an agent may legitimately leave unset.
 */
function agentConfigRunError(args: Record<string, any> | undefined): string | undefined {
	const provider = args?.provider
	if (!provider?.resource || !provider?.model) {
		return 'Select a provider resource and model before deploying.'
	}
	// `kind` is what the worker deserializes the provider as, and a config written through the JSON
	// editor can leave it out entirely. Presence only: the list of kinds lives behind the provider
	// SDKs, and an unrecognised one already fails the same way at the worker.
	if (typeof provider.kind !== 'string' || provider.kind === '') {
		return 'The provider is missing its kind. Pick the resource again to set it.'
	}
	// Reported rather than iterated: the editor renders a JSON-authored non-list as an empty
	// roster, so this is the only place left that would meet the raw value, and a bare `.filter`
	// on it throws past the caller. `null` is how the API spells "unset".
	if (args?.tools != null && !Array.isArray(args.tools)) {
		return 'Tools must be a list. Fix it in the resource editor before deploying.'
	}
	// Only a flowmodule tool's summary is a callable name: the worker never reads an mcp or
	// websearch summary, so a blank one there is not an error and must not count as a sibling.
	// Same filter as `collectInvalidAgentToolNames`, which is the rule the graph enforces.
	const tools = (args?.tools ?? []) as Record<string, any>[]
	const named = tools.filter(
		(t) => t?.value?.tool_type !== 'mcp' && t?.value?.tool_type !== 'websearch'
	)
	const siblingNames = named.map((t) => t?.summary ?? '')
	for (const tool of named) {
		const err = getToolNameError(tool?.summary ?? '', tool?.value?.tool_type, siblingNames)
		if (err) return `${err}. Name every tool before deploying.`
	}
	// An MCP tool is named by its server rather than by a summary, and it starts with none. The
	// worker loads every MCP config before the model runs, so one unresolved path fails every run
	// of the agent rather than just the call that needed it.
	if (tools.some((t) => t?.value?.tool_type === 'mcp' && !t?.value?.resource_path)) {
		return 'Pick a server for every MCP tool before deploying.'
	}
	return undefined
}

/**
 * The draft shape every resource editor writes, and the only one the review/deploy page knows how
 * to canonicalize and deploy (`utils_draft_deploy.ts`, `canonicalizeDraftDiffValue` /
 * `deployDraft`). The agent editor is a different *form* over this same row, never a parallel
 * draft: writing anything else here would take agent drafts off the deploy page and out of the
 * generic editor.
 *
 * `args` is the resource value. `description`, `labels` and `wsSpecific` are round-tripped
 * untouched — the agent form has no place for them, and dropping them would rewrite the resource
 * on deploy.
 */
export interface AgentResourceState {
	path: string
	description: string
	args: AIAgentConfig
	/** Present only for a path with no deployed row, whose type the draft is the sole record of.
	 *  Elsewhere it is left out to match the generic resource editor, which has no field for it. */
	resource_type?: string
	labels?: string[]
	wsSpecific: boolean
}

export interface AgentDraftOptions {
	/** The `ai_agent` resource being edited. */
	path: () => string | undefined
	workspace: () => string | undefined
}

export interface AgentDraftHandle {
	/** The form's live state, draft included. Bind the editor to `state.args`. */
	readonly state: AgentResourceState | undefined
	/** The deployed baseline, for the banner's diff. */
	readonly deployed: AgentResourceState | undefined
	readonly loading: boolean
	/** No deployed row at this path, so a deploy has to create rather than update. */
	readonly noDeployed: boolean
	/** Whether this user may write the resource. False makes the editor a read-only view. */
	readonly canWrite: boolean
	/** Why this path cannot be edited here, if it cannot. Render it instead of the form. */
	readonly refusal: string | undefined
	readonly sync: TriggerDraftSync
	/** Write the current state to the resource and drop the draft. */
	deploy: () => Promise<boolean>
}

/**
 * Local-autosave wiring for an `ai_agent` resource, over the shared drawer-kind draft sync. Holding
 * a live handle is what makes a write from elsewhere — another tab, the generic resource editor on
 * the same path — land in the open agent editor.
 */
export function useAgentDraft(opts: AgentDraftOptions): AgentDraftHandle {
	let state = $state<AgentResourceState | undefined>(undefined)
	let deployed = $state<AgentResourceState | undefined>(undefined)
	let loading = $state(true)
	let noDeployed = $state(false)
	let canWriteResource = $state(true)
	/** Guards the load against a path that changed under a slow response. */
	let loadedFor = $state<string | undefined>(undefined)
	/** Why the loaded path cannot be edited here, if it cannot. Gates the sync for as long as that
	 *  path stays, so a resource this editor refuses can neither restore its draft into the form nor
	 *  have one autosaved from it. Set it through `refuse`, which also clears what was loaded. */
	let refusal = $state<string | undefined>(undefined)

	const sync = useTriggerDraftSync({
		itemKind: 'resource',
		path: () => opts.path() ?? '',
		workspace: () => opts.workspace(),
		drawerLoading: () => loading || refusal != null,
		// `$state.snapshot` deep-reads, so the sync effects re-run when a nested field of `args`
		// changes. Returning the object itself only subscribes to the reference, and every edit the
		// form makes is a nested one.
		getCfg: () => (state ? ($state.snapshot(state) as Record<string, any>) : undefined),
		applyCfg: (cfg) => {
			state = cfg as AgentResourceState
		},
		deployed: () => deployed as Record<string, any> | undefined
	})

	function refuse(reason: string) {
		loading = false
		refusal = reason
		state = undefined
		deployed = undefined
		noDeployed = false
		canWriteResource = false
		sendUserToast(reason, true)
	}

	$effect(() => {
		const path = opts.path()
		const ws = opts.workspace()
		untrack(() => {
			if (!path || !ws) {
				state = undefined
				deployed = undefined
				loadedFor = undefined
				refusal = undefined
				return
			}
			const key = `${ws}:${path}`
			if (loadedFor === key) return
			loadedFor = key
			loading = true
			refusal = undefined
			// The user alongside the resource, as the generic resource editor loads it: a session or
			// fork editor operates on a workspace that is not the one being navigated, and groups,
			// folders and the admin flag are all per workspace, so the nav user would answer for the
			// wrong membership in both directions.
			Promise.all([
				ResourceService.getResource({ workspace: ws, path, getDraft: true }),
				getUserExt(ws).catch(() => undefined)
			])
				// The rejection handler is `then`'s second argument rather than a trailing `catch`, so
				// it answers for the fetch alone: a throw in the body below leaves a loaded editor
				// standing instead of tearing it down as an unreadable resource.
				.then(
					async ([r, user]) => {
						// A slower response for a path we have left must not overwrite the current one.
						if (loadedFor !== key) return
						// A step's `agent` is caller-authored, so it can name a resource of any type, and a
						// deploy from here would replace that resource's whole value while keeping its type.
						const refused = agentEditorRefusal(path, r.resource_type)
						if (refused) {
							refuse(refused)
							return
						}
						noDeployed = Boolean((r as any).no_deployed)
						const deployedState: AgentResourceState = {
							path: r.path,
							description: r.description ?? '',
							args: (r.value ?? {}) as AIAgentConfig,
							// Only where nothing else answers for the type: with a deployed row both the
							// create below and the review page's `deployDraft` read it from there, and
							// carrying it would make every draft the generic resource editor writes — which
							// has no field for it — differ from this baseline on that key alone.
							...(noDeployed ? { resource_type: r.resource_type } : {}),
							labels: r.labels ?? undefined,
							wsSpecific: r.ws_specific ?? false
						}
						// Same rule the generic resource editor applies. The backend refuses the write
						// either way, but without this the editor would invite edits it cannot save and
						// autosave a draft on every keystroke against a resource the reader cannot deploy.
						canWriteResource = canWrite(
							r.path,
							r.extra_perms ?? {},
							user ?? get(userStore) ?? undefined
						)
						// An agent that exists only as a draft has no deployed value to compare against or
						// fall back to, and the response's is a synthetic echo of the draft. Leaving the
						// baseline unset is what suppresses the unsaved-changes banner for it, exactly as
						// `useTriggerDraftSync` intends: its Discard would otherwise reset to that synthetic
						// value and delete the one row the agent lives in.
						deployed = noDeployed ? undefined : deployedState
						// Open on the draft when there is one, so the editor never flashes the deployed
						// config before the autosave lands.
						state =
							((r as any).draft as AgentResourceState | undefined) ?? structuredClone(deployedState)
						loading = false
						await sync.maybeRestore()
					},
					(err) => {
						// A failed load knows neither the resource's type nor its value, so it refuses:
						// clearing `loading` alone would let the sync restore a persisted draft into a form
						// that would then deploy over a resource nobody read.
						if (loadedFor !== key) return
						refuse(`Could not load agent ${path}: ${err}`)
					}
				)
		})
	})

	async function deploy(): Promise<boolean> {
		const ws = opts.workspace()
		const s = state
		if (!ws || !s) return false
		// The editor offers only values, but a transform can arrive from a step that was forked
		// before this existed, or from the generic resource editor: say so rather than writing it.
		const transformValued = transformValuedBrainKeys(s.args)
		if (transformValued.length > 0) {
			const fields = transformValued.map((key) => AGENT_BRAIN_LABELS[key] ?? key)
			const many = fields.length > 1
			sendUserToast(
				`${fields.join(', ')} ${many ? 'are' : 'is'} set to an expression or an AI-filled value, which a saved agent cannot store. Replace ${many ? 'them' : 'it'} with a plain value before deploying.`,
				true
			)
			return false
		}
		// The resource endpoint takes any JSON, so nothing downstream stops an agent that cannot run:
		// the worker needs a provider to call and rejects a tool whose name it cannot pass to the
		// model. Deploying one would break every flow linking it, so it is refused here.
		const blocked = agentConfigRunError(s.args)
		if (blocked) {
			sendUserToast(blocked, true)
			return false
		}
		// Renaming is not this editor's to do: moving the resource leaves every step that links to it
		// naming a path that no longer exists, and reconciling those is a feature of its own. A
		// renamed path can still reach here, the generic editor writing the same draft row and
		// offering a path field, so refuse it rather than performing half of a rename.
		const currentPath = opts.path()
		if (currentPath && s.path !== currentPath) {
			sendUserToast(
				`This draft renames the agent to ${s.path}. Deploy it from the resource editor instead.`,
				true
			)
			return false
		}
		// Only a draft naming another type: the load refuses a resource that is not an agent, while a
		// draft the generic resource editor wrote names no type at all and inherits the loaded one.
		if (s.resource_type && s.resource_type !== 'ai_agent') {
			sendUserToast(`This draft is a ${s.resource_type} resource, not an agent.`, true)
			return false
		}
		// The form stays editable while the request is in flight, so everything below works from a
		// snapshot taken now. Adopting the live state as `deployed` afterwards would count an edit
		// made during the request as saved, and the banner would clear on a value the server never
		// received; against the snapshot it stays a draft, which is what it is.
		const submitted = structuredClone($state.snapshot(s)) as AgentResourceState
		const body = {
			path: submitted.path,
			value: submitted.args,
			description: submitted.description,
			labels: submitted.labels,
			ws_specific: submitted.wsSpecific
		}
		try {
			if (noDeployed) {
				await ResourceService.createResource({
					workspace: ws,
					// A create needs a type, and the load proved this path is an agent before opening.
					requestBody: { ...body, resource_type: submitted.resource_type ?? 'ai_agent' }
				})
			} else {
				// The type this editor proved is as old as the load, and an update carries no type of
				// its own: were the path deleted and recreated as something else meanwhile, this write
				// would put an agent config inside that resource. Reading it again narrows the window
				// to the request rather than to however long the editor stayed open.
				const current = await ResourceService.getResource({ workspace: ws, path: submitted.path })
				const refused = agentEditorRefusal(submitted.path, current.resource_type)
				if (refused) {
					refuse(refused)
					return false
				}
				await ResourceService.updateResource({
					workspace: ws,
					path: submitted.path,
					requestBody: body
				})
			}
		} catch (err) {
			sendUserToast(`Could not save agent: ${err}`, true)
			return false
		}
		// The counter the step card's write-back used to report, from the surface that now owns the
		// write: a deploy here reaches every flow linking this agent.
		logReusableAgentUsage(noDeployed ? 'saved' : 'updated')
		deployed = submitted
		noDeployed = false
		// Only when the form still holds exactly what was sent. `discard` resets the handle's cell to
		// what it is given, and the apply-effect copies that back over the form: against an edit made
		// while the request was in flight that would erase it, draft and all. Such an edit is a real
		// unsaved change over the version just deployed, so it keeps its draft and its banner.
		if (!deepEqual($state.snapshot(state), submitted)) {
			sendUserToast(`Saved agent ${submitted.path}. Later edits are still unsaved`)
			loadedFor = `${ws}:${submitted.path}`
			return true
		}
		// `discard`, not `remove`: it resets the handle's cell to what was just saved, so the
		// apply-effect cannot bounce the form back to the now-stale draft.
		sync.discard(opts.path()!, submitted)
		// A rename moves the row, so the next load must not reuse the old key.
		loadedFor = `${ws}:${submitted.path}`
		sendUserToast(`Saved agent ${submitted.path}`)
		return true
	}

	return {
		get state() {
			return state
		},
		get deployed() {
			return deployed
		},
		get loading() {
			return loading
		},
		get noDeployed() {
			return noDeployed
		},
		get canWrite() {
			return canWriteResource
		},
		get refusal() {
			return refusal
		},
		sync,
		deploy
	}
}
