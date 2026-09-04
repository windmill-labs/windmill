<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, ChevronDown, ChevronUp, Save, Unlink, Pencil } from 'lucide-svelte'
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
	import { agentWriteCount, markAgentWritten, openAgentEditor } from '../agentEditorStore.svelte'
	import {
		setLinkedAgentTools,
		clearLinkedAgentTools,
		linkedToolsScope
	} from '../linkedAgentToolsStore.svelte'
	import { logReusableAgentUsage } from '../agentTelemetry'
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
		flowPath = '',
		fromAgentEditor = false
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
		// Inside the agent editor, where an agent used as a tool stays part of the agent being
		// edited: it cannot be saved as a reusable agent of its own, and one already linked cannot be
		// opened here. Linking a saved agent to another agent is out of scope for this editor — the
		// backend supports it, but only a flow can author it, and a second editor over a second draft
		// is the wrong way in.
		fromAgentEditor?: boolean
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)

	// How many times the linked agent has been written, from anywhere: this card's own save, or a
	// deploy from the agent editor mounted alongside it. Both reads below key on it, so neither
	// keeps naming the config and version a write has just replaced.
	let writes = $derived(agentWriteCount(ws, agent))

	let saveDrawer: Drawer | undefined = $state()
	let newPath = $state('')
	let pathError = $state('')
	let description = $state('')
	let saving = $state(false)

	type LinkedInfo = {
		// What this result was fetched for. runed's resource neither aborts nor tags a superseded
		// request, so a slow fetch can land after a newer one: every consumer gates on these matching
		// the current (ws, agent, writes). `writes` is what covers a refetch of the *same* link after
		// a deploy — without it a pre-deploy response is indistinguishable from the current one, and
		// accepting it republishes the tools the deploy just replaced.
		ws?: string
		path?: string
		writes: number
		config: AIAgentConfig
		tools: AgentTool[]
		providerPath?: string
		providerOk: boolean
	}

	// A linked agent is rigid and read-only: its brain and tools come from the resource. We
	// load them here for display, and probe the provider resource so we can warn when it isn't
	// accessible in this workspace (the user then needs to unlink/fork or gain access).
	let linkedResource = resource(
		() => ({ ws, path: agent, writes }),
		async ({ ws, path, writes }): Promise<LinkedInfo> => {
			if (!ws || !path) {
				return { ws, path, writes, config: {}, tools: [], providerOk: true }
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
			return {
				ws,
				path,
				writes,
				config: cfg,
				tools,
				providerPath,
				providerOk
			}
		}
	)
	// Retain the last result that matched the current link. Discarding a superseded one outright
	// would blank the card, because a stale response for a previous agent replaces
	// `linkedResource.current` and nothing refetches the one actually linked.
	let loadedInfo = $state<LinkedInfo | undefined>(undefined)
	$effect(() => {
		const current = linkedResource.current
		if (current && current.ws === ws && current.path === agent && current.writes === writes) {
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
	let cardPath = $derived(agent)
	// The version eval runs are recorded against. The resource does not hold it; its newest history
	// entry does, since recording is a database trigger on every write.
	let versionResource = resource(
		() => ({ ws, path: cardPath, writes }),
		async ({
			ws,
			path,
			writes
		}): Promise<{ ws?: string; path?: string; writes: number; version?: number }> => {
			if (!ws || !path) {
				return { ws, path, writes }
			}
			const history = await ResourceService.getResourceHistory({ workspace: ws, path })
			return { ws, path, writes, version: history.versions?.[0]?.version }
		}
	)
	// Guarded like the link above, `writes` included: a response for a previous agent must not label
	// this one, and one from before a deploy must not relabel it with the version it replaced.
	let version = $derived.by(() => {
		const loaded = versionResource.current
		return loaded !== undefined &&
			loaded.ws === ws &&
			loaded.path === cardPath &&
			loaded.writes === writes
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
			setLinkedAgentTools(toolScope, moduleId, loaded.tools as AgentToolStrict[], agent)
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

	let showDetail = $state(false)

	function openSave() {
		newPath = ''
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
		// If the step is replaced while the requests below are in flight (undo, a session-draft sync),
		// the resource is still written but the step must not be relinked and emptied. The tools array
		// this save started from is what identifies it, and its link answers for the case where a
		// step keeps that array yet is pointed at an agent of its own meanwhile.
		const forkMarker = tools
		const startedUnlinkedFrom = agent
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
		markAgentWritten(ws, path)
		if (tools !== forkMarker || agent !== startedUnlinkedFrom) {
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
			const linked = await persist(newPath, description)
			saveDrawer?.closeDrawer()
			if (linked) {
				logReusableAgentUsage('saved')
				sendUserToast(`Saved reusable agent ${newPath}`)
			}
		} catch (e) {
			sendUserToast(`Failed to save agent: ${e}`, true)
		} finally {
			saving = false
		}
	}

	// Copy the resource's brain + tools into the step, so it can diverge from the agent it was
	// linked to. This flow's tool_inputs are folded into the tools and then cleared, so the
	// standalone step keeps the bindings it was running with.
	// Returns false when the step changed under the fetch, so the caller can say nothing happened.
	async function forkFromResource(): Promise<boolean> {
		if (!ws || !agent) {
			return false
		}
		const path = agent
		// `tools` is one array per module value, so it identifies the step itself — the path alone
		// would not, since a replacement can carry the same link.
		const stepMarker = tools
		const res = await ResourceService.getResource({ workspace: ws, path })
		// The module may have been replaced while the fetch was in flight (undo, session drafts);
		// applying a stale fork would overwrite the restored state.
		if (agent !== path || tools !== stepMarker) {
			return false
		}
		const cfg = (res.value ?? {}) as AIAgentConfig
		// Preserve the flow-local inputs already wired in the step.
		const local: Record<string, InputTransform> = {}
		for (const key of AGENT_FLOW_LOCAL_KEYS) {
			if (inputTransforms?.[key]) {
				local[key] = inputTransforms[key]
			}
		}
		const forkedInputs = { ...agentConfigToInputTransforms(cfg), ...local }
		const forkedTools = cfg.tools ?? []
		inputTransforms = forkedInputs
		for (const tool of forkedTools) {
			const overrides = toolInputs?.[tool.id]
			if (overrides && tool.value?.input_transforms) {
				tool.value.input_transforms = { ...tool.value.input_transforms, ...overrides }
			}
		}
		toolInputs = {}
		tools = forkedTools
		agent = undefined
		return true
	}

	// Unlink forks the agent into this step so it can diverge here. It does not write back.
	async function unlink() {
		try {
			const forked = await forkFromResource()
			if (forked) {
				logReusableAgentUsage('unlinked')
				sendUserToast('Forked agent. Its configuration was copied into this step')
			} else {
				sendUserToast('The step changed while loading the agent, so nothing was unlinked', true)
			}
		} catch (e) {
			sendUserToast(`Failed to unlink agent: ${e}`, true)
		}
	}

	// Edit the saved agent itself. The step stays linked throughout: the edits live in the agent's
	// own resource draft, not in this step, so they survive leaving the flow and are the same edits
	// whichever flow — or the resources page — opened them.
	function editAgent() {
		if (!agent) return
		openAgentEditor({
			path: agent,
			workspace: ws,
			// Where to re-resolve this graph's tool nodes once the agent is deployed.
			host: { flowPath, moduleId }
		})
	}
</script>

<div class="px-2 xl:px-4 py-1.5 border-b border-light">
	{#if agent}
		<div class="rounded-md border border-light bg-surface-tertiary px-3 py-2">
			<!-- The whole line toggles the detail; the link and the buttons stop propagation. -->
			<div
				class="flex items-center gap-2 cursor-pointer"
				role="button"
				tabindex="0"
				aria-expanded={showDetail}
				onclick={() => (showDetail = !showDetail)}
				onkeydown={(e) => {
					// Keys aimed at the buttons inside the row bubble through here; leave them theirs.
					if (e.target !== e.currentTarget) return
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault()
						showDetail = !showDetail
					}
				}}
			>
				<Bot size={16} class="text-primary shrink-0" />
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
						<span class="text-tertiary">
							{#if showDetail}
								<ChevronUp size={14} />
							{:else}
								<ChevronDown size={14} />
							{/if}
						</span>
					{/if}
					{#if !fromAgentEditor}
						<Button
							unifiedSize="sm"
							variant="default"
							startIcon={{ icon: Pencil }}
							iconOnly
							title="Edit the saved agent (updates it everywhere it's used)"
							onclick={(e) => {
								e.stopPropagation()
								editAgent()
							}}
						/>
					{/if}
					<Button
						unifiedSize="sm"
						variant="default"
						startIcon={{ icon: Unlink }}
						iconOnly
						title="Unlink (fork an editable copy into just this step)"
						onclick={(e) => {
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
	{:else if !fromAgentEditor}
		<Button
			unifiedSize="sm"
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
