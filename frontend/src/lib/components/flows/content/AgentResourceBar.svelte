<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import LightweightResourcePicker from '$lib/components/LightweightResourcePicker.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, Save, Unlink, Pencil } from 'lucide-svelte'
	import {
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
	import type { AgentTool as AgentToolStrict } from '../agentToolUtils'
	import { resource } from 'runed'

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
	let pickerValue: string | undefined = $state(undefined)
	// Set when the step was forked from a saved agent for editing: it's now an editable standalone
	// step, but "Save changes" upserts back to this path (and re-links), propagating to all flows.
	// Kept in an external store (not local state) so it survives this component unmounting when
	// another node is selected; validated against the forked `tools` identity so a stale entry
	// can't resurface after undo/session restores or in another flow (see agentEditStore).
	let editingPath = $derived(getAgentEditingPath(tools))

	type LinkedInfo = {
		config: AIAgentConfig
		tools: AgentTool[]
		providerPath?: string
		providerOk: boolean
	}

	// A linked agent is rigid and read-only: its brain, tools and evals come from the resource. We
	// load them here for display, and probe the provider resource so we can warn when it isn't
	// accessible in this workspace (the user then needs to unlink/fork or gain access).
	let linkedResource = resource(
		() => ({ ws, path: agent }),
		async ({ ws, path }): Promise<LinkedInfo> => {
			if (!ws || !path) {
				return { config: {}, tools: [], providerOk: true }
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
			return { config: cfg, tools, providerPath, providerOk }
		}
	)
	let inheritedTools = $derived(linkedResource.current?.tools ?? [])
	let brainParams = $derived(summarizeAgentBrain(linkedResource.current?.config))
	let providerPath = $derived(linkedResource.current?.providerPath)
	let providerOk = $derived(linkedResource.current?.providerOk ?? true)

	// Keep the graph's linked-tool store current for this step. flowState resolves every linked step
	// at load; here we refresh the one being edited when its link changes (or clear it on unlink), so
	// its tool nodes update without reloading the flow.
	let toolScope = $derived(linkedToolsScope(ws, flowPath))
	$effect(() => {
		if (!agent) {
			clearLinkedAgentTools(toolScope, moduleId)
			return
		}
		// Publish only once the resource has loaded — don't clobber the load-time tools with [] while
		// the fetch is in flight, which would flicker the tool nodes out of the graph on selection.
		const loaded = linkedResource.current
		if (loaded) {
			// linkedResource types tools loosely; they are the same resource tools the store holds.
			setLinkedAgentTools(toolScope, moduleId, loaded.tools as AgentToolStrict[])
		}
	})

	// Link the step as soon as a saved agent is picked. Linking is rigid, so the step keeps no tools
	// of its own — they come from the resource. The picked value is consumed immediately: a stale
	// pickerValue must not re-link over an external change to `agent` (undo, session drafts).
	$effect(() => {
		if (pickerValue && pickerValue !== agent) {
			agent = pickerValue
			pickerValue = undefined
			tools = []
			toolInputs = {}
			// Drop the step's brain transforms; a linked step keeps only the flow-local inputs.
			inputTransforms = flowLocalInputs(inputTransforms)
		}
	})

	function toolLabel(tool: AgentTool): string {
		return tool.summary || tool.value?.tool_type || tool.id
	}

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

	// Create or update the `ai_agent` resource at `path` from the step's current brain + tools, then
	// link the step to it.
	async function persist(path: string, description?: string) {
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
		agent = path
		// The brain + tools now live in the resource; a linked step keeps only the flow-local inputs.
		tools = []
		inputTransforms = flowLocalInputs(inputTransforms)
		setAgentEditingPath(tools, undefined)
	}

	async function saveAsAgent() {
		if (!ws || pathError || !newPath) {
			return
		}
		saving = true
		try {
			const updating = newPath === editingPath
			await persist(newPath, description)
			saveDrawer?.closeDrawer()
			sendUserToast(updating ? `Updated agent ${newPath}` : `Saved reusable agent ${newPath}`)
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
			await persist(path)
			sendUserToast(`Updated agent ${path}`)
		} catch (e) {
			sendUserToast(`Failed to update agent: ${e}`, true)
		} finally {
			saving = false
		}
	}

	// Copy the linked resource's brain + tools into the step, turning it back into a standalone
	// editable agent. Shared by Unlink (diverge here) and Edit (edit the saved agent itself).
	// Unlink folds the host flow's tool_inputs overrides into the tools (the standalone step keeps
	// its effective bindings) and clears them. Edit must NOT fold: it edits the shared defaults, so
	// it starts from the resource's own tools and preserves tool_inputs so this flow's overrides
	// survive the re-link after Save changes instead of being promoted into the agent.
	async function forkFromResource(foldOverrides: boolean): Promise<string | undefined> {
		if (!ws || !agent) {
			return undefined
		}
		const path = agent
		const res = await ResourceService.getResource({ workspace: ws, path })
		// The module may have been replaced while the fetch was in flight (undo, session drafts);
		// applying a stale fork would overwrite the restored state and recreate the Editing target.
		if (agent !== path) {
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
		pickerValue = undefined
		return path
	}

	// Unlink forks the agent into this step so it can diverge here. It does not write back.
	async function unlink() {
		try {
			const path = await forkFromResource(true)
			if (path) {
				setAgentEditingPath(tools, undefined)
				sendUserToast('Forked agent — its configuration was copied into this step')
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
				sendUserToast(`Editing ${path} — make changes, then Save changes to update it`)
			}
		} catch (e) {
			sendUserToast(`Failed to edit agent: ${e}`, true)
		}
	}

	// Cancel keeps the edits as a standalone step. Edit preserved the flow-local overrides for the
	// re-link; on a standalone step the runtime ignores tool_inputs, so fold them into the tools
	// (as Unlink does) to keep the step's effective bindings, and clear the now-inert map.
	function cancelEdit() {
		for (const tool of tools) {
			const overrides = toolInputs?.[tool.id]
			if (overrides && tool.value?.input_transforms) {
				tool.value.input_transforms = { ...tool.value.input_transforms, ...overrides }
			}
		}
		toolInputs = {}
		setAgentEditingPath(tools, undefined)
	}
</script>

<div class="px-2 xl:px-4 pt-2">
	{#if agent}
		<div class="rounded-md border border-light bg-surface-secondary px-3 py-2">
			<div class="flex items-center gap-2 text-xs">
				<Bot size={16} class="text-primary shrink-0" />
				<span class="text-secondary shrink-0">Linked to</span>
				<span class="flex min-w-0 flex-1 items-center gap-1">
					<a
						class="font-medium truncate"
						href={`/resources?path=${agent}&workspace=${ws}`}
						title={agent}>{agent}</a
					>
					<Tooltip small placement="bottom">
						{#snippet text()}
							Read-only: the configuration comes from this saved agent, and only the message and
							inputs are set in this flow. Edit changes the agent everywhere it's used. Unlink forks
							an editable copy into just this step.
						{/snippet}
					</Tooltip>
				</span>
				<div class="ml-auto flex items-center gap-1 shrink-0">
					<Button
						size="xs2"
						variant="default"
						startIcon={{ icon: Pencil }}
						iconOnly
						title="Edit the saved agent (updates it everywhere it's used)"
						onclick={editAgent}
					/>
					<Button
						size="xs2"
						variant="default"
						startIcon={{ icon: Unlink }}
						iconOnly
						title="Unlink (fork an editable copy into just this step)"
						onclick={unlink}
					/>
				</div>
			</div>
			{#if brainParams.length > 0 || inheritedTools.length > 0}
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
									<span
										class="inline-flex items-center rounded border border-light bg-surface px-1.5 py-0.5 text-secondary"
										title={tool.id}
									>
										{toolLabel(tool)}
									</span>
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
		<div
			class="flex items-center gap-2 rounded-md border border-light bg-surface-secondary px-3 py-2 text-xs"
		>
			<Pencil size={16} class="text-primary shrink-0" />
			<span class="text-secondary">Editing</span>
			<span class="font-medium truncate" title={editingPath}>{editingPath}</span>
			<div class="ml-auto flex items-center gap-1">
				<Button
					size="xs2"
					variant="accent"
					startIcon={{ icon: Save }}
					disabled={saving || !!providerSaveError}
					onclick={saveChanges}
				>
					Save changes
				</Button>
				<Button size="xs2" variant="default" onclick={cancelEdit}>Cancel</Button>
			</div>
		</div>
		<p class="text-2xs text-tertiary mt-1">
			Editing the saved agent. Save changes updates it and re-links this step — the update
			propagates to every flow that links to it. Cancel keeps your edits here as a standalone step
			instead.
		</p>
		{#if providerSaveError}
			<p class="text-2xs text-red-600 dark:text-red-400 mt-1">
				{providerSaveError}
			</p>
		{/if}
	{:else}
		<div class="flex items-center gap-2">
			<div class="grow min-w-0">
				<LightweightResourcePicker
					bind:value={pickerValue}
					resourceType="ai_agent"
					workspace={ws}
				/>
			</div>
			<span class="text-2xs text-tertiary">or</span>
			<Button size="xs2" variant="default" startIcon={{ icon: Save }} onclick={openSave}>
				Save as agent
			</Button>
		</div>
	{/if}
</div>

<Drawer bind:this={saveDrawer} size="600px">
	<DrawerContent title="Save as reusable agent" on:close={() => saveDrawer?.closeDrawer()}>
		<div class="flex flex-col gap-4">
			<p class="text-xs text-secondary">
				Save this AI agent's configuration and tools as a reusable resource. Other flows can then
				link to it, and updates propagate automatically.
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
