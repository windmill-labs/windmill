<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ResourceService, type InputTransform, type Resource } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Bot, ChevronDown, ChevronUp, FlaskConical, Save, Unlink, Pencil } from 'lucide-svelte'
	import AgentEvalModal from '$lib/components/aiEvals/AgentEvalModal.svelte'
	import LocalDraftBanner from '$lib/components/LocalDraftBanner.svelte'
	import DraftSyncConflictModal from '$lib/components/common/confirmationModal/DraftSyncConflictModal.svelte'
	import {
		AGENT_BRAIN_KEYS,
		AGENT_FLOW_LOCAL_KEYS,
		agentConfigToInputTransforms,
		flowLocalInputs,
		inputTransformsToAgentConfig,
		nonStaticBrainKeys,
		summarizeAgentBrain,
		type AIAgentConfig,
		type AgentTool
	} from '../agentResourceUtils'
	import {
		setLinkedAgentTools,
		clearLinkedAgentTools,
		linkedToolsScope
	} from '../linkedAgentToolsStore.svelte'
	import { getAgentEditingPath, setAgentEditingPath } from '../agentEditStore.svelte'
	import { claimLinkedToolsFetch } from '../flowState'
	import type { AgentTool as AgentToolStrict } from '../agentToolUtils'
	import { resource } from 'runed'
	import { untrack } from 'svelte'

	let {
		agent = $bindable(),
		inputTransforms = $bindable(),
		tools = $bindable(),
		toolInputs = $bindable(),
		moduleId,
		opWorkspace = undefined,
		flowPath = ''
	}: {
		agent: string | undefined
		inputTransforms: Record<string, InputTransform>
		tools: AgentTool[]
		toolInputs: Record<string, Record<string, InputTransform>>
		moduleId: string
		// The workspace the flow editor operates on (differs from the nav workspace in session/fork
		// editors). All resource reads/writes must target it, not $workspaceStore.
		opWorkspace?: string
		// Scope for the linked-agent tools store (the flow path); must match what the graph reads.
		flowPath?: string
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)

	let saveDrawer: Drawer | undefined = $state()
	let newPath = $state('')
	let pathError = $state('')
	let description = $state('')
	let saving = $state(false)
	// The path "Save changes" upserts back to, for a step forked from a saved agent. Lives in an
	// external store so it survives this component unmounting when another node is selected, keyed
	// by the forked `tools` identity so a stale entry can't resurface (see agentEditStore).
	let editingPath = $derived(getAgentEditingPath(tools))

	type LinkedInfo = {
		// What this result was fetched for. runed's resource neither aborts nor tags a superseded
		// request, so a slow fetch for a previous link can land after a newer one: every consumer
		// gates on these matching the current (ws, agent).
		ws?: string
		path?: string
		config: AIAgentConfig
		tools: AgentTool[]
		providerPath?: string
		providerOk: boolean
		hasDraft: boolean
	}

	// A linked agent is rigid and read-only: its brain and tools come from the resource. We
	// load them here for display, and probe the provider resource so we can warn when it isn't
	// accessible in this workspace (the user then needs to unlink/fork or gain access).
	let linkedResource = resource(
		() => ({ ws, path: agent }),
		async ({ ws, path }): Promise<LinkedInfo> => {
			if (!ws || !path) {
				return { ws, path, config: {}, tools: [], providerOk: true, hasDraft: false }
			}
			// `getDraft` for the overlay, not for the value: the card is about the deployed agent, and
			// `value` stays deployed either way. Cancel leaves a draft behind on purpose, and a card
			// that never mentions it is a card whose Evals button offers to run edits nothing named.
			const res = (await ResourceService.getResource({
				workspace: ws,
				path,
				getDraft: true
			})) as Resource & { draft_saved_at?: string }
			const cfg = (res.value ?? {}) as AIAgentConfig & { provider?: { resource?: string } }
			const tools = (cfg.tools ?? []) as AgentTool[]
			const providerRef = cfg.provider?.resource
			const providerPath =
				typeof providerRef === 'string' && providerRef
					? providerRef.replace(/^\$res:/, '').replace(/^res:\/\//, '')
					: undefined
			let providerOk = true
			if (providerPath) {
				try {
					await ResourceService.getResource({ workspace: ws, path: providerPath })
				} catch {
					providerOk = false
				}
			}
			return {
				ws,
				path,
				config: cfg,
				tools,
				providerPath,
				providerOk,
				hasDraft: res.draft_saved_at != undefined
			}
		}
	)
	// Retain the last result that matched the current link. Discarding a superseded one outright
	// would blank the card, because a stale response for a previous agent replaces
	// `linkedResource.current` and nothing refetches the one actually linked.
	let loadedInfo = $state<LinkedInfo | undefined>(undefined)
	$effect(() => {
		const current = linkedResource.current
		if (current && current.ws === ws && current.path === agent) {
			loadedInfo = current
		}
	})
	let linkedInfo = $derived(
		loadedInfo?.ws === ws && loadedInfo?.path === agent ? loadedInfo : undefined
	)
	let inheritedTools = $derived(linkedInfo?.tools ?? [])
	let brainParams = $derived(summarizeAgentBrain(linkedInfo?.config))
	let providerPath = $derived(linkedInfo?.providerPath)
	let providerOk = $derived(linkedInfo?.providerOk ?? true)
	/** The agent the card is about: the one this step links to, or the one being edited. */
	let cardPath = $derived(agent ?? editingPath)
	/** That agent's draft as the syncer keys it. Keyed on the card's agent rather than the editor's,
	 *  so a write that only fails once the editor is gone is still answered for. */
	let cardDraftQuery = $derived(
		ws && cardPath ? { workspace: ws, itemKind: 'resource' as const, path: cardPath } : undefined
	)
	let cardDraftSync = $derived(
		cardDraftQuery ? UserDraftDbSyncer.getState(cardDraftQuery) : undefined
	)
	/** Why the edits are not on the agent, when they are not. Cancel is only safe to press because
	 *  the draft is holding them, so a mirror that failed has to say so — and go on saying so after
	 *  Cancel, which is the point at which the work is gone for good. A conflict is the other way it
	 *  stops, and the modal below owns that one. */
	let draftSyncFailure = $derived(
		cardDraftSync?.state === 'failed'
			? (cardDraftSync.failureMessage ?? 'Unknown error')
			: undefined
	)
	/** A save the server refused because the draft had moved on. Read here as well as in the modal:
	 *  a conflict is reported as a snapshot rather than as a sync state, so watching the state alone
	 *  misses it — and it can be raised by a write still in flight when the editor closes, which is
	 *  the case that would otherwise pass in silence. */
	let cardDraftConflict = $derived(
		cardDraftQuery ? UserDraftDbSyncer.getConflict(cardDraftQuery) : undefined
	)
	/** The edits never reached the agent's draft, whichever way it stopped. */
	let draftNotWritten = $derived(
		draftSyncFailure !== undefined || cardDraftConflict?.conflict !== undefined
	)
	/** The last thing this card did to an agent's draft, and to which agent. Writes and deletes are
	 *  queued, so the card can re-read the resource while one is still on its way: what was done
	 *  here is the more current answer until the card links elsewhere, when it stops being about
	 *  this agent at all. */
	let draftLeftHere = $state<{ path: string; has: boolean } | undefined>(undefined)
	/** The agent has edits nobody has deployed — left by Cancel here, or written in the resource
	 *  editor. Named on the card because it is what the Evals button offers to run. A write this
	 *  card queued and then lost is not a draft: once it has failed or been refused, the server's
	 *  answer is the true one again, or the badge would claim edits the agent never received. */
	let linkedHasDraft = $derived(
		draftLeftHere != undefined &&
			agent != undefined &&
			draftLeftHere.path === agent &&
			!draftNotWritten
			? draftLeftHere.has
			: (linkedInfo?.hasDraft ?? false)
	)
	/** The write that failed was the delete, so the draft is still there rather than missing. Both
	 *  go through one syncer key, and the two say opposite things to whoever reads the card. */
	let discardFailed = $derived(
		draftSyncFailure !== undefined &&
			draftLeftHere?.path === cardPath &&
			draftLeftHere?.has === false
	)
	// Evals belong to the agent, not to the step, so they open over the flow rather than as one of
	// the step's tabs: what you go there to read is a history of runs, not a setting of this step.
	let evalsOpen = $state(false)
	// Bumped on every write to the resource, so a save that leaves the card on the same agent still
	// refetches the version it just minted.
	let writes = $state(0)
	// An eval run is recorded against a version, so the card names the one this agent's runs will
	// carry. The resource does not hold it; its newest history entry does, because recording is a
	// database trigger on every write.
	let versionResource = resource(
		() => ({ ws, path: cardPath, writes }),
		async ({ ws, path }): Promise<{ ws?: string; path?: string; version?: number }> => {
			if (!ws || !path) {
				return { ws, path }
			}
			const history = await ResourceService.getResourceHistory({ workspace: ws, path })
			return { ws, path, version: history.versions?.[0]?.version }
		}
	)
	// Guarded like the link above: a response for a previous agent must not label this one.
	let version = $derived.by(() => {
		const loaded = versionResource.current
		return loaded !== undefined && loaded.ws === ws && loaded.path === cardPath
			? loaded.version
			: undefined
	})

	// Keep the graph's linked-tool store current for this step. flowState resolves every linked step
	// at load; here we refresh the one being edited when its link changes (or clear it on unlink), so
	// its tool nodes update without reloading the flow.
	let toolScope = $derived(linkedToolsScope(ws, flowPath))
	// The agent the store's tools currently belong to, so a link change can be told apart from a step
	// whose tools were resolved at flow load. Seeded from the link at mount, because initFlowState
	// has already published for it — leaving it unset would miss a link change that lands before this
	// component's own request. Deliberately not reactive: it tracks what was written.
	let publishedFor: string | undefined = untrack(() => agent)
	$effect(() => {
		if (!agent) {
			// Claim first: an in-flight fetch for the previous link would otherwise still pass its own
			// generation check and re-add the tools we just cleared.
			claimLinkedToolsFetch(toolScope, moduleId)
			clearLinkedAgentTools(toolScope, moduleId)
			publishedFor = undefined
			return
		}
		const loaded = linkedInfo
		if (loaded) {
			claimLinkedToolsFetch(toolScope, moduleId)
			// linkedResource types tools loosely; they are the same resource tools the store holds.
			setLinkedAgentTools(toolScope, moduleId, loaded.tools as AgentToolStrict[])
			publishedFor = agent
		} else if (publishedFor !== undefined && publishedFor !== agent) {
			// The link moved and the new agent hasn't resolved, so the stored tools are the old one's.
			// No claim: writing `agent` re-runs the editor's watcher, which already superseded the old
			// fetch and started one for the new link — claiming would discard it, leaving the graph
			// empty unless this step stays selected. Load-time tools stay put (publishedFor unset).
			clearLinkedAgentTools(toolScope, moduleId)
			publishedFor = undefined
		}
	})

	function toolLabel(tool: AgentTool): string {
		return tool.summary || tool.value?.tool_type || tool.id
	}

	// What the agent is, rather than which agent it is: a strip that sits above every tab says the
	// second by default and the first when asked.
	let showDetail = $state(false)

	function openSave() {
		newPath = editingPath ?? ''
		pathError = ''
		description = ''
		saveDrawer?.openDrawer()
	}

	// The provider is required by the backend (AIAgentArgs.provider is non-optional), so an agent
	// saved without a complete one fails on every linked run. Block saving when the provider is
	// computed/connected (only a static value can be captured into the resource) or when the static
	// value is incomplete (a fresh step defaults to empty resource/model, which is still static).
	let providerSaveError = $derived.by(() => {
		const t = inputTransforms?.provider as
			| { type?: string; value?: { resource?: string; model?: string } }
			| undefined
		if (!t || t.type !== 'static') {
			return 'Set a static provider before saving. A linked agent needs a provider stored on the resource, so a computed/connected value can not be saved.'
		}
		if (!t.value?.resource || !t.value?.model) {
			return 'Select a provider resource and model before saving.'
		}
		return undefined
	})

	// What linking throws away: the brain transforms (static or not) and the step's own tools. The
	// flow-local inputs survive linking, so a change to those must not block it.
	function discardedOnLinkSnapshot(): string {
		const brain: Record<string, unknown> = {}
		for (const key of AGENT_BRAIN_KEYS) {
			if (inputTransforms?.[key] !== undefined) {
				brain[key] = inputTransforms[key]
			}
		}
		return JSON.stringify([brain, tools])
	}

	// Create or update the `ai_agent` resource at `path` from the step's current brain + tools, then
	// link the step to it.
	// Returns false when the resource was written but the step was left alone, so callers can skip
	// the success toast that would otherwise bury the explanation.
	async function persist(path: string, description?: string): Promise<boolean> {
		const dropped = nonStaticBrainKeys(inputTransforms)
		if (providerSaveError) {
			throw new Error(providerSaveError)
		}
		if (dropped.length > 0) {
			sendUserToast(
				`Note: ${dropped.join(', ')} use a computed/connected value and won't be saved into the agent`,
				true
			)
		}
		// Tool inputs are saved verbatim: the agent carries its tools' default bindings (static, AI or
		// flow expressions) as authored. Host flows override per-step via tool_inputs, never here.
		const value = inputTransformsToAgentConfig(inputTransforms, tools)
		// The editor stays live during the requests below, so remember what linking would discard:
		// every brain transform and the tools. Comparing the saved config instead would miss a
		// non-static brain edit, which the resource cannot hold yet linking still strips.
		const savedSnapshot = discardedOnLinkSnapshot()
		// If the edit session ends or changes while the requests below are in flight (Cancel, undo,
		// session-draft sync, a different agent opened for editing), the resource is still written but
		// the step must not be relinked/cleared. Pinning the path — not merely "some edit is active" —
		// is what distinguishes this session from a replacement one.
		const forkMarker = tools
		const savingEditPath = getAgentEditingPath(forkMarker)
		const exists = await ResourceService.existsResource({ workspace: ws!, path })
		if (exists) {
			// The drawer's path check is debounced, so a fast save can reach here with an unrelated
			// resource at the path — never clobber a resource of another type.
			const existing = await ResourceService.getResource({ workspace: ws!, path })
			if (existing.resource_type !== 'ai_agent') {
				throw new Error(
					`A ${existing.resource_type} resource already exists at ${path}. Pick another path.`
				)
			}
			await ResourceService.updateResourceValue({
				workspace: ws!,
				path,
				requestBody: { value }
			})
		} else {
			await ResourceService.createResource({
				workspace: ws!,
				requestBody: {
					path,
					value,
					resource_type: 'ai_agent',
					description: description || 'Reusable AI agent'
				}
			})
		}
		// The write minted a version, and nothing else the fetch keys on has to change for it to be
		// the one the card should now be naming.
		writes++
		// Editing: a content-preserving refresh may have re-anchored the marker onto a clone of
		// `tools`, which is still this session; a cleared or different path is not. Saving a
		// standalone step has no marker to track, so only the fork's own array identifies it.
		const sameSession =
			savingEditPath === undefined
				? tools === forkMarker
				: getAgentEditingPath(tools) === savingEditPath
		if (!sameSession) {
			// The resource is written either way; say so, or the drawer just closes with no outcome.
			sendUserToast(
				`Saved ${path}, but the step changed while saving, so it was not linked to the agent`,
				true
			)
			return false
		}
		// Edits made while the save was in flight aren't in the resource; linking now would strip them
		// from the step too, losing them entirely. Keep the step as-is and let the user save again.
		if (discardedOnLinkSnapshot() !== savedSnapshot) {
			sendUserToast(
				`Saved ${path}, but changes made while saving are not in it. Save again to include them`,
				true
			)
			return false
		}
		agent = path
		// Clear the edit entry while `tools` is still the fork's marker, before it's reassigned.
		setAgentEditingPath(tools, undefined)
		setAgentEditingPath(forkMarker, undefined)
		// The brain + tools now live in the resource; a linked step keeps only the flow-local inputs.
		tools = []
		inputTransforms = flowLocalInputs(inputTransforms)
		return true
	}

	async function saveAsAgent() {
		if (!ws || pathError || !newPath) {
			return
		}
		saving = true
		try {
			const updating = newPath === editingPath
			const linked = await persist(newPath, description)
			saveDrawer?.closeDrawer()
			if (linked) {
				sendUserToast(updating ? `Updated agent ${newPath}` : `Saved reusable agent ${newPath}`)
			}
		} catch (e) {
			sendUserToast(`Failed to save agent: ${e}`, true)
		} finally {
			saving = false
		}
	}

	// Save the forked-for-edit step back to the agent it came from, updating it in place.
	async function saveChanges() {
		if (!ws || !editingPath) {
			return
		}
		saving = true
		const path = editingPath
		try {
			if (await persist(path)) {
				// Deployed: the draft described what is now the agent, so it describes nothing.
				clearDraft(path)
				sendUserToast(`Updated agent ${path}`)
			}
		} catch (e) {
			sendUserToast(`Failed to update agent: ${e}`, true)
		} finally {
			saving = false
		}
	}

	// Copy the resource's brain + tools into the step, for Unlink (diverge here) and Edit (change the
	// saved agent). Unlink folds this flow's tool_inputs into the tools and clears them, so the
	// standalone step keeps its bindings; Edit must not fold, or those overrides would be promoted
	// into the shared agent instead of surviving the re-link.
	async function forkFromResource(
		foldOverrides: boolean,
		resumeDraft = false
	): Promise<string | undefined> {
		if (!ws || !agent) {
			return undefined
		}
		const path = agent
		// `tools` is one array per module value, so it identifies the step itself — the path alone
		// would not, since a replacement can carry the same link.
		const stepMarker = tools
		const draftKey = { workspace: ws, itemKind: 'resource' as const, path }
		if (resumeDraft) {
			// A write can still be queued here: Cancel leaves the debounce running, which is what
			// makes "Cancel keeps your work" true, and reopening within it must not lose the last
			// edit. Sent before the read, so the read returns it, and while `last_sync` is still the
			// one it was written against — the server judges it on its own baseline, landing an edit
			// made a moment ago and refusing again one it has already refused. Only a write that
			// survives that is left parked, and the read below is what earns the right to drop it.
			try {
				await UserDraftDbSyncer.flush(draftKey)
			} catch {
				// Reported by the card's own failure alert; all that matters here is it did not land.
			}
		}
		// `getDraft` rather than a second request for the draft: the overlay rides the load that
		// already has to succeed, so a failure cannot quietly mean "no draft" and let the next
		// keystroke write over one. It carries `draft_saved_at` too, which is the baseline a later
		// autosave sends as `last_sync`, so a newer draft from another tab is rejected rather than
		// overwritten.
		const res = (await ResourceService.getResource({
			workspace: ws,
			path,
			getDraft: resumeDraft
		})) as Resource & { draft?: { args?: AIAgentConfig }; draft_saved_at?: string }
		// The module may have been replaced while the fetch was in flight (undo, session drafts);
		// applying a stale fork would overwrite the restored state and recreate the Editing target.
		if (agent !== path || tools !== stepMarker) {
			return undefined
		}
		const cfg = (res.value ?? {}) as AIAgentConfig
		// Preserve the flow-local inputs already wired in the step.
		const local: Record<string, InputTransform> = {}
		for (const key of AGENT_FLOW_LOCAL_KEYS) {
			if (inputTransforms?.[key]) {
				local[key] = inputTransforms[key]
			}
		}
		// What the agent holds, as the same round trip the mirror below writes: opening the editor
		// normalises the configuration, and a normalisation is not an edit. Comparing against the
		// resource's own JSON would compare key order too.
		deployedConfig = JSON.stringify(
			inputTransformsToAgentConfig(
				{ ...agentConfigToInputTransforms(cfg), ...local },
				cfg.tools ?? []
			)
		)

		// Editing resumes this user's own unsaved work when there is any. Opening on the deployed
		// value would show them something other than what they last wrote, and then write over it
		// on the first keystroke.
		let source = cfg
		if (resumeDraft) {
			// The overlay files the value under `args`, the shape the resource editor writes, so both
			// editors read and write the same draft.
			const draftCfg = res.draft?.args
			if (draftCfg) {
				source = draftCfg
			}
			// Whatever is still parked was composed against an older copy than the one just read, so
			// flushing it later would put content the editor no longer shows over the draft on
			// screen. Dropped after the read rather than before it: until the read lands, this is
			// the only copy of those edits, and a read that fails has to leave them recoverable.
			UserDraftDbSyncer.dropPending(draftKey)
			UserDraftDbSyncer.recordRemoteSync(draftKey, res.draft_saved_at)
		}
		const brain = agentConfigToInputTransforms(source)
		inputTransforms = { ...brain, ...local }
		const forkedTools = source.tools ?? []
		// What was written into the step, which is what the mirror waits to see before it treats a
		// difference as an edit. Not the same as `deployedConfig` once a draft has been resumed.
		forkedConfig = JSON.stringify(inputTransformsToAgentConfig(inputTransforms, forkedTools))
		if (foldOverrides) {
			for (const tool of forkedTools) {
				const overrides = toolInputs?.[tool.id]
				if (overrides && tool.value?.input_transforms) {
					tool.value.input_transforms = { ...tool.value.input_transforms, ...overrides }
				}
			}
			toolInputs = {}
		}
		tools = forkedTools
		agent = undefined
		return path
	}

	// Unlink forks the agent into this step so it can diverge here. It does not write back.
	async function unlink() {
		try {
			const path = await forkFromResource(true)
			if (path) {
				setAgentEditingPath(tools, undefined)
				sendUserToast('Forked agent. Its configuration was copied into this step')
			} else {
				sendUserToast('The step changed while loading the agent, so nothing was unlinked', true)
			}
		} catch (e) {
			sendUserToast(`Failed to unlink agent: ${e}`, true)
		}
	}

	// Edit the saved agent itself: fork it into the step for editing, remembering the path so
	// "Save changes" writes back to it (updating every flow that links to it).
	async function editAgent() {
		try {
			const path = await forkFromResource(false, true)
			if (path) {
				setAgentEditingPath(tools, path)
				sendUserToast(`Editing ${path}. Make changes, then Save changes to update it`)
			} else {
				sendUserToast('The step changed while loading the agent. Try Edit again', true)
			}
		} catch (e) {
			sendUserToast(`Failed to edit agent: ${e}`, true)
		}
	}

	/**
	 * Mirror the edit in progress into the agent's own draft.
	 *
	 * The step is forked while you edit it, which is what makes the edits runnable here — but the
	 * agent is what is being edited, so that is where the unsaved state belongs. Kept there, it
	 * survives closing the flow, shows the agent as drafted wherever it appears, and is what evals
	 * run when you ask them to run the draft rather than what is deployed.
	 */
	function mirrorEditToDraft(path: string) {
		if (!ws) return
		UserDraftDbSyncer.save({ workspace: ws, itemKind: 'resource', path, value: draftValue(path) })
		draftLeftHere = { path, has: true }
	}

	/** What is filed as the draft: the configuration under `args`, wrapped the way the resource
	 *  editor wraps it, which is the shape both editors and the run path read it back from. */
	function draftValue(path: string) {
		return {
			path,
			description: '',
			args: inputTransformsToAgentConfig(inputTransforms, tools),
			labels: undefined,
			wsSpecific: false
		}
	}

	/** Drop it: deployed, undone or discarded, the draft describes nothing the agent does not
	 *  already hold. */
	function clearDraft(path: string) {
		if (!ws) return
		UserDraftDbSyncer.save({ workspace: ws, itemKind: 'resource', path, value: null })
		draftLeftHere = { path, has: false }
	}

	$effect(() => {
		const path = editingPath
		// Read so an edit to either re-runs this.
		const config = JSON.stringify(inputTransformsToAgentConfig(inputTransforms, tools))
		untrack(() => {
			if (!path || deployedConfig === undefined) return
			// The fork is written into the step through bound props, so it arrives over several
			// states: the first ones read back are the step part-way through being forked, not
			// anything anyone edited. It has landed once what the step holds is what was forked
			// into it, and only from there is a difference an edit.
			if (!forkSettled) {
				if (config !== forkedConfig) return
				forkSettled = true
				// The baseline every later difference is measured against, so a resumed draft is not
				// immediately rewritten with the same content it was read from.
				mirrored = config
				return
			}
			if (config === mirrored) return
			const had = mirrored !== undefined && mirrored !== deployedConfig
			mirrored = config
			if (config !== deployedConfig) {
				mirrorEditToDraft(path)
			} else if (had) {
				// Back at the deployed value, so there is nothing unsaved left to keep — including a
				// draft this editor resumed rather than wrote, which is the rule the resource editor
				// applies to its own.
				clearDraft(path)
			}
		})
	})
	/** The agent as deployed, and the last state compared against it. */
	let deployedConfig: string | undefined = $state(undefined)
	let mirrored: string | undefined = $state(undefined)
	/** What was written into the step when the editor opened — the draft when one was resumed,
	 *  otherwise the deployed value. What the settle gate waits to see. */
	let forkedConfig: string | undefined = $state(undefined)
	/** The step is holding the configuration that was forked into it, so what it holds from here is
	 *  what someone did to it. */
	let forkSettled = $state(false)
	/** Whether what is in the editor differs from the agent it is an edit of. */
	let edited = $derived(
		deployedConfig !== undefined && mirrored !== undefined && mirrored !== deployedConfig
	)
	$effect(() => {
		if (!editingPath) {
			untrack(() => {
				deployedConfig = undefined
				mirrored = undefined
				forkedConfig = undefined
				forkSettled = false
			})
		}
	})

	/** The draft the editor is on, as the syncer keys it. */
	let draftQuery = $derived(
		ws && editingPath
			? { workspace: ws, itemKind: 'resource' as const, path: editingPath }
			: undefined
	)
	/**
	 * Take the draft as the server has it, discarding what this editor holds — the conflict modal's
	 * "Load from server". Re-enters the editor rather than writing the values into the open one,
	 * whose panes are mounted on the fork and would keep painting what was replaced.
	 */
	async function reloadFromServer() {
		if (!editingPath) return
		// The refused write is dropped by the re-fork below, which is where every read of the
		// server's copy drops it — reopening the editor after a Cancel takes the same path.
		cancelEdit()
		try {
			const path = await forkFromResource(false, true)
			if (path) {
				setAgentEditingPath(tools, path)
			}
		} catch (e) {
			sendUserToast(`Failed to reload the agent: ${e}`, true)
		}
	}

	/**
	 * Drop the draft and put the editor back on what is deployed, which is what the banner's
	 * Discard means everywhere else it appears.
	 */
	function discardDraft() {
		const path = editingPath
		if (!path) return
		clearDraft(path)
		// And leave the editor, which re-links the step to the deployed agent — the baseline being
		// reset to. Reloading the values into an editor that is already open would leave its panes
		// painting what was just discarded until it is reopened.
		cancelEdit()
	}

	// Cancel discards the edits and re-links the step, leaving the agent untouched. Diverging from
	// the agent is Unlink's job, on the linked card. Edit kept this flow's `tool_inputs` off the
	// forked tools rather than folding them in, so they survive the round trip as overrides.
	function cancelEdit() {
		const path = editingPath
		// Clear the entry while `tools` is still the fork's array, which is what keys it.
		setAgentEditingPath(tools, undefined)
		if (!path) {
			return
		}
		// The draft stays. Cancel un-forks the step; it is not a decision about the unsaved work,
		// which is what Discard on the banner is for — the same split the resource editor makes.
		// Edits that ended back at the deployed value are already cleared by the mirror above.
		agent = path
		tools = []
		inputTransforms = flowLocalInputs(inputTransforms)
	}
</script>

<div class="px-2 xl:px-4 py-1.5 border-b border-light">
	{#if agent}
		<div class="rounded-md border border-light bg-surface-tertiary px-3 py-2">
			<!-- The line is the control: clicking it says what the agent is. Only the path leaves for
			     the resource, and the buttons do their own thing, so both stop here. -->
			<div
				class="flex items-center gap-2 cursor-pointer"
				role="button"
				tabindex="0"
				aria-expanded={showDetail}
				onclick={() => (showDetail = !showDetail)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault()
						showDetail = !showDetail
					}
				}}
			>
				<Bot size={16} class="text-primary shrink-0" />
				<!-- The link is as wide as the path and no wider: the rest of the line belongs to the
				     card, which is what makes clicking it expand rather than navigate. -->
				<div class="min-w-0 flex-1 flex items-center gap-2">
					<a
						class="truncate text-xs font-medium hover:underline"
						href={`/resources?path=${agent}&workspace=${ws}`}
						title={`Open ${agent}`}
						onclick={(e) => e.stopPropagation()}>{agent}</a
					>
					{#if version != undefined}
						<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
							v{version}
						</Badge>
					{/if}
					{#if linkedHasDraft}
						<Badge
							color="yellow"
							class="shrink-0"
							title="This agent has undeployed edits. Edit continues them, and Evals offers to run them."
						>
							unsaved changes
						</Badge>
					{/if}
				</div>
				<div class="flex items-center gap-1 shrink-0">
					{#if brainParams.length > 0 || inheritedTools.length > 0}
						<!-- Kept beside the line it opens: a card that only expands when you happen to click
						     it is a card nobody knows expands. -->
						<span class="text-tertiary">
							{#if showDetail}
								<ChevronUp size={14} />
							{:else}
								<ChevronDown size={14} />
							{/if}
						</span>
					{/if}
					<Button
						unifiedSize="2xs"
						variant="default"
						startIcon={{ icon: FlaskConical }}
						title="Run this agent against a dataset of cases"
						on:click={(e) => {
							e.stopPropagation()
							evalsOpen = true
						}}
					>
						Evals
					</Button>
					<Button
						unifiedSize="2xs"
						variant="default"
						startIcon={{ icon: Pencil }}
						iconOnly
						title="Edit the saved agent (updates it everywhere it's used)"
						on:click={(e) => {
							e.stopPropagation()
							editAgent()
						}}
					/>
					<Button
						unifiedSize="2xs"
						variant="default"
						startIcon={{ icon: Unlink }}
						iconOnly
						title="Unlink (fork an editable copy into just this step)"
						on:click={(e) => {
							e.stopPropagation()
							unlink()
						}}
					/>
				</div>
			</div>
			{#if showDetail && (brainParams.length > 0 || inheritedTools.length > 0)}
				<dl class="mt-2 flex flex-col gap-1">
					{#each brainParams as param (param.label)}
						<div class="flex items-baseline gap-2 text-2xs">
							<dt class="text-tertiary shrink-0 w-28">{param.label}</dt>
							<dd class="text-secondary truncate" title={param.value}>{param.value}</dd>
						</div>
					{/each}
					{#if inheritedTools.length > 0}
						<div class="flex items-baseline gap-2 text-2xs">
							<dt class="text-tertiary shrink-0 w-28">Tools</dt>
							<dd class="flex flex-wrap gap-1">
								{#each inheritedTools as tool (tool.id)}
									<Badge color="gray" title={tool.id}>{toolLabel(tool)}</Badge>
								{/each}
							</dd>
						</div>
					{/if}
				</dl>
			{/if}
		</div>
		{#if !providerOk}
			<div class="mt-1">
				<Alert type="error" size="xs" title="Model provider not accessible">
					This agent's model provider{#if providerPath}
						(<span class="font-medium">{providerPath}</span>){/if} isn't accessible in this workspace.
					Unlink to fork the agent, or gain access to the provider resource.
				</Alert>
			</div>
		{/if}
	{:else if editingPath}
		<!-- What saving does is the consequence worth reading, so it is under the agent it is about
		     rather than under an icon. The buttons sit against both lines. -->
		<div
			class="rounded-md border border-light bg-surface-tertiary px-3 py-1.5 flex flex-col gap-1.5"
		>
			<div class="flex items-center gap-2">
				<Pencil size={16} class="text-primary shrink-0" />
				<div class="min-w-0 flex-1 flex flex-col">
					<div class="flex items-center gap-2 min-w-0">
						<span class="truncate text-xs font-medium" title={editingPath}>{editingPath}</span>
						{#if version != undefined}
							<Badge color="gray" class="shrink-0" title="The version these edits sit on">
								v{version}
							</Badge>
						{/if}
						{#if edited}
							<Badge color="yellow">unsaved changes</Badge>
						{/if}
					</div>
					<div class="text-2xs text-secondary flex items-center gap-0.5">
						saving updates every flow using it<Tooltip small>
							{#snippet text()}
								Edits are kept on the agent as a draft, so they survive leaving this flow, and
								opening Edit again continues them. Save changes writes them back to the agent and
								re-links this step; Cancel re-links it and leaves the draft for next time. Discard,
								on the banner, is what drops the draft.
							{/snippet}
						</Tooltip>
					</div>
				</div>
				<!-- Beside what it is about: evals of an agent being edited run the edits, so it
				     belongs to the line naming them rather than to the row that keeps or discards
				     them. -->
				<Button
					unifiedSize="2xs"
					variant="default"
					startIcon={{ icon: FlaskConical }}
					title="Run these edits against a dataset of cases"
					onclick={() => (evalsOpen = true)}
				>
					Evals
				</Button>
			</div>
			<!-- The same banner the resource editor shows over a draft, so unsaved work reads the
			     same wherever it is met: what differs from deployed, and the one control that drops
			     it. Editing resumes a draft rather than replacing it, which is exactly the state
			     worth naming. -->
			<LocalDraftBanner
				show={edited}
				getDeployed={() => (deployedConfig ? JSON.parse(deployedConfig) : undefined)}
				getCurrent={() => inputTransformsToAgentConfig(inputTransforms, tools)}
				onDiscard={discardDraft}
				title="Deployed <> Unsaved agent changes"
			/>
			<!-- Deciding the edits' fate gets a row of its own: at this width it was wrapping into
			     the line that names them, and the two are not the same question. -->
			<div class="flex items-center justify-end gap-1">
				<Button unifiedSize="2xs" variant="default" onclick={cancelEdit}>Cancel</Button>
				<Button
					unifiedSize="2xs"
					variant="accent"
					startIcon={{ icon: Save }}
					disabled={saving || !!providerSaveError}
					onclick={saveChanges}
				>
					Save changes
				</Button>
			</div>
		</div>
		{#if providerSaveError}
			<p class="text-2xs text-red-500 mt-1">
				{providerSaveError}
			</p>
		{/if}
	{:else}
		<Button
			unifiedSize="2xs"
			variant="default"
			startIcon={{ icon: Save }}
			wrapperClasses="w-full"
			btnClasses="w-full"
			onclick={openSave}
		>
			Save as reusable agent
		</Button>
	{/if}
	<!-- Outside the editing card on purpose: the write outlives the editor, and Cancel unmounting
	     the only account of a failed one is what leaves the step claiming work it lost. -->
	{#if draftNotWritten}
		<div class="mt-1">
			{#if discardFailed}
				<!-- Both writes this card makes share one key, so a failed delete arrives here too —
				     saying the edits are missing while the badge above correctly says they are there.
				     Worded for the delete itself rather than for why it was made, which is deploying,
				     discarding or editing back to the deployed value. -->
				<Alert type="error" size="xs" title="This agent's draft was not removed">
					Removing it did not reach the server: {draftSyncFailure}. The agent goes on reading as
					having unsaved changes until it is removed.
				</Alert>
			{:else if draftSyncFailure}
				<Alert type="error" size="xs" title="These edits are not on the agent">
					They could not be written to its draft: {draftSyncFailure}. Save changes, while the step
					is still open, writes them to the agent instead.
				</Alert>
			{:else}
				<Alert type="error" size="xs" title="These edits are not on the agent">
					A newer draft of this agent was saved elsewhere, so these were not written over it. The
					agent's draft, and anything run against it, is that other one.
				</Alert>
			{/if}
		</div>
	{/if}
</div>

<Drawer bind:this={saveDrawer} size="600px">
	<DrawerContent title="Save as reusable agent" on:close={() => saveDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Save this AI agent's configuration and tools as a reusable resource. Other flows can then
				link to it, updates propagate automatically, and it gains a dataset of eval cases of its
				own.
			</p>
			<Path
				bind:path={newPath}
				bind:error={pathError}
				initialPath=""
				namePlaceholder="my_agent"
				kind="resource"
				workspaceOverride={ws}
			/>
			<label class="flex flex-col gap-1 text-xs">
				<span class="text-secondary">Description</span>
				<TextInput
					bind:value={description}
					inputProps={{ placeholder: 'What this agent does' }}
					size="sm"
				/>
			</label>
			{#if providerSaveError}
				<p class="text-xs text-red-600 dark:text-red-400">
					{providerSaveError}
				</p>
			{/if}
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				startIcon={{ icon: Save }}
				disabled={!newPath || !!pathError || saving || !!providerSaveError}
				onclick={saveAsAgent}
			>
				Save agent
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<AgentEvalModal agentPath={cardPath} {opWorkspace} bind:open={evalsOpen} />

<!-- The agent's draft is shared with the resource editor, so it can advance under this one. The
     modal is the shared answer to that: it holds the screen until the two are reconciled, rather
     than letting the mirror stall behind a card that still reads as saved. -->
{#if draftQuery}
	<DraftSyncConflictModal
		query={draftQuery}
		onLoadFromServer={reloadFromServer}
		getLocalDraft={() => (editingPath ? draftValue(editingPath) : undefined)}
	/>
{/if}
