import { untrack } from 'svelte'
import { ResourceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { useTriggerDraftSync, type TriggerDraftSync } from '../triggers/useTriggerDraftSync.svelte'
import { AGENT_BRAIN_KEYS, type AIAgentConfig } from './agentResourceUtils'

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
	/** Carried so the deploy page's diff doesn't read the draft as dropping it, and so a
	 *  draft-only agent creates with the right type. */
	resource_type: string
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
	/** Guards the load against a path that changed under a slow response. */
	let loadedFor = $state<string | undefined>(undefined)

	const sync = useTriggerDraftSync({
		itemKind: 'resource',
		path: () => opts.path() ?? '',
		workspace: () => opts.workspace(),
		drawerLoading: () => loading,
		// `$state.snapshot` deep-reads, so the sync effects re-run when a nested field of `args`
		// changes. Returning the object itself only subscribes to the reference, and every edit the
		// form makes is a nested one.
		getCfg: () => (state ? ($state.snapshot(state) as Record<string, any>) : undefined),
		applyCfg: (cfg) => {
			state = cfg as AgentResourceState
		},
		deployed: () => deployed as Record<string, any> | undefined
	})

	$effect(() => {
		const path = opts.path()
		const ws = opts.workspace()
		untrack(() => {
			if (!path || !ws) {
				state = undefined
				deployed = undefined
				loadedFor = undefined
				return
			}
			const key = `${ws}:${path}`
			if (loadedFor === key) return
			loadedFor = key
			loading = true
			ResourceService.getResource({ workspace: ws, path, getDraft: true })
				.then(async (r) => {
					// A slower response for a path we have left must not overwrite the current one.
					if (loadedFor !== key) return
					const deployedState: AgentResourceState = {
						path: r.path,
						description: r.description ?? '',
						args: (r.value ?? {}) as AIAgentConfig,
						resource_type: r.resource_type ?? 'ai_agent',
						labels: r.labels ?? undefined,
						wsSpecific: r.ws_specific ?? false
					}
					deployed = deployedState
					noDeployed = Boolean((r as any).no_deployed)
					// Open on the draft when there is one, so the editor never flashes the deployed
					// config before the autosave lands.
					state =
						((r as any).draft as AgentResourceState | undefined) ?? structuredClone(deployedState)
					loading = false
					await sync.maybeRestore()
				})
				.catch((err) => {
					if (loadedFor !== key) return
					loading = false
					sendUserToast(`Could not load agent ${path}: ${err}`, true)
				})
		})
	})

	async function deploy(): Promise<boolean> {
		const ws = opts.workspace()
		const s = state
		if (!ws || !s) return false
		// A saved agent's config is plain JSON, so a brain field holding a transform rather than a
		// value cannot be written. The editor offers only static values, but one can arrive from a
		// step that was forked before this existed, or from the generic resource editor — say so
		// rather than dropping it on the floor.
		const nonStatic = AGENT_BRAIN_KEYS.filter((key) => {
			const v = (s.args as Record<string, unknown>)[key]
			return (
				v !== null && typeof v === 'object' && 'type' in (v as object) && 'expr' in (v as object)
			)
		})
		if (nonStatic.length > 0) {
			sendUserToast(
				`${nonStatic.join(', ')} hold an expression, which a saved agent cannot store. Replace them with values before deploying.`,
				true
			)
			return false
		}
		const body = {
			path: s.path,
			value: s.args,
			description: s.description,
			labels: s.labels,
			ws_specific: s.wsSpecific
		}
		try {
			if (noDeployed) {
				await ResourceService.createResource({
					workspace: ws,
					requestBody: { ...body, resource_type: s.resource_type }
				})
			} else {
				await ResourceService.updateResource({
					workspace: ws,
					path: opts.path()!,
					requestBody: body
				})
			}
		} catch (err) {
			sendUserToast(`Could not save agent: ${err}`, true)
			return false
		}
		const saved = structuredClone($state.snapshot(s)) as AgentResourceState
		deployed = saved
		noDeployed = false
		// `discard`, not `remove`: it resets the handle's cell to what was just saved, so the
		// apply-effect cannot bounce the form back to the now-stale draft.
		sync.discard(opts.path()!, saved)
		// A rename moves the row, so the next load must not reuse the old key.
		loadedFor = `${ws}:${saved.path}`
		sendUserToast(`Saved agent ${saved.path}`)
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
		sync,
		deploy
	}
}
