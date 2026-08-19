<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Bot, ChevronDown, ChevronUp, FlaskConical, Save, Unlink, Pencil } from 'lucide-svelte'
	import AgentEvalModal from '$lib/components/aiEvals/AgentEvalModal.svelte'
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
	}

	// A linked agent is rigid and read-only: its brain and tools come from the resource. We
	// load them here for display, and probe the provider resource so we can warn when it isn't
	// accessible in this workspace (the user then needs to unlink/fork or gain access).
	let linkedResource = resource(
		() => ({ ws, path: agent }),
		async ({ ws, path }): Promise<LinkedInfo> => {
			if (!ws || !path) {
				return { ws, path, config: {}, tools: [], providerOk: true }
			}
			const res = await ResourceService.getResource({ workspace: ws, path })
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
			return { ws, path, config: cfg, tools, providerPath, providerOk }
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
	async function forkFromResource(foldOverrides: boolean): Promise<string | undefined> {
		if (!ws || !agent) {
			return undefined
		}
		const path = agent
		// `tools` is one array per module value, so it identifies the step itself — the path alone
		// would not, since a replacement can carry the same link.
		const stepMarker = tools
		const res = await ResourceService.getResource({ workspace: ws, path })
		// The module may have been replaced while the fetch was in flight (undo, session drafts);
		// applying a stale fork would overwrite the restored state and recreate the Editing target.
		if (agent !== path || tools !== stepMarker) {
			return undefined
		}
		const cfg = (res.value ?? {}) as AIAgentConfig
		const brain = agentConfigToInputTransforms(cfg)
		// Preserve the flow-local inputs already wired in the step.
		const local: Record<string, InputTransform> = {}
		for (const key of AGENT_FLOW_LOCAL_KEYS) {
			if (inputTransforms?.[key]) {
				local[key] = inputTransforms[key]
			}
		}
		inputTransforms = { ...brain, ...local }
		const forkedTools = cfg.tools ?? []
		// What the agent holds, as the same round trip the mirror below writes: opening the editor
		// normalises the configuration, and a normalisation is not an edit. Comparing against the
		// resource's own JSON would compare key order too.
		deployedConfig = JSON.stringify(inputTransformsToAgentConfig(inputTransforms, forkedTools))
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
			const path = await forkFromResource(false)
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
		UserDraftDbSyncer.save({
			workspace: ws,
			itemKind: 'resource',
			path,
			value: {
				path,
				description: '',
				args: inputTransformsToAgentConfig(inputTransforms, tools),
				labels: undefined,
				wsSpecific: false
			}
		})
	}

	/** Drop it: the edit was saved, so the draft describes nothing that is not deployed, or it was
	 *  abandoned, so it describes nothing at all. */
	function clearDraft(path: string) {
		if (!ws) return
		UserDraftDbSyncer.save({ workspace: ws, itemKind: 'resource', path, value: null })
	}

	$effect(() => {
		const path = editingPath
		// Read so an edit to either re-runs this.
		const config = JSON.stringify(inputTransformsToAgentConfig(inputTransforms, tools))
		untrack(() => {
			if (!path || deployedConfig === undefined || config === mirrored) return
			const had = mirrored !== undefined && mirrored !== deployedConfig
			mirrored = config
			if (config !== deployedConfig) {
				mirrorEditToDraft(path)
			} else if (had) {
				// The edits were undone. Only what was written here is dropped: a draft that was already
				// there is someone's unsaved work, and this opened on the deployed value, not on it.
				clearDraft(path)
			}
		})
	})
	/** The agent as deployed, and the last state compared against it. */
	let deployedConfig: string | undefined = $state(undefined)
	let mirrored: string | undefined = $state(undefined)
	/** Whether what is in the editor differs from the agent it is an edit of. */
	let edited = $derived(
		deployedConfig !== undefined && mirrored !== undefined && mirrored !== deployedConfig
	)
	$effect(() => {
		if (!editingPath) {
			untrack(() => {
				deployedConfig = undefined
				mirrored = undefined
			})
		}
	})

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
		clearDraft(path)
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
						size="xs2"
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
						size="xs2"
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
						size="xs2"
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
				<dl class="mt-2 flex flex-col gap-1 border-t border-light pt-2">
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
								Edits are kept on the agent as a draft, so they survive leaving this flow. Save
								changes writes them back to the agent and re-links this step. Cancel discards them
								and re-links it unchanged.
							{/snippet}
						</Tooltip>
					</div>
				</div>
				<!-- Beside what it is about: evals of an agent being edited run the edits, so it
				     belongs to the line naming them rather than to the row that keeps or discards
				     them. -->
				<Button
					size="xs2"
					variant="default"
					startIcon={{ icon: FlaskConical }}
					title="Run these edits against a dataset of cases"
					onclick={() => (evalsOpen = true)}
				>
					Evals
				</Button>
			</div>
			<!-- Deciding the edits' fate gets a row of its own: at this width it was wrapping into
			     the line that names them, and the two are not the same question. -->
			<div class="flex items-center justify-end gap-1">
				<Button size="xs2" variant="default" onclick={cancelEdit}>Cancel</Button>
				<Button
					size="xs2"
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
			size="xs2"
			variant="default"
			startIcon={{ icon: Save }}
			wrapperClasses="w-full"
			btnClasses="w-full"
			onclick={openSave}
		>
			Save as reusable agent
		</Button>
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
