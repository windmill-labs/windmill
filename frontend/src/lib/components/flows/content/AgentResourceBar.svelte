<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Path from '$lib/components/Path.svelte'
	import LightweightResourcePicker from '$lib/components/LightweightResourcePicker.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, Save, Unlink, Pencil, TriangleAlert } from 'lucide-svelte'
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
	import { setLinkedAgentTools, clearLinkedAgentTools } from '../linkedAgentToolsStore.svelte'
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
	let editingPath: string | undefined = $state(undefined)

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

	// A linked tool's inputs come from the resource, where they reference the *authoring* flow. Any
	// dynamic (computed/connected) input the user hasn't rebound in this step still points at the
	// source flow and silently evaluates wrong here — count them so we can warn to rebind.
	let unboundToolInputs = $derived.by(() => {
		let count = 0
		for (const tool of inheritedTools) {
			const value = tool.value as
				| { tool_type?: string; input_transforms?: Record<string, InputTransform> }
				| undefined
			if (value?.tool_type !== 'flowmodule') continue
			const overrides = toolInputs?.[tool.id] ?? {}
			for (const [key, t] of Object.entries(value.input_transforms ?? {})) {
				if ((t as { type?: string })?.type !== 'static' && !(key in overrides)) {
					count++
				}
			}
		}
		return count
	})

	// Keep the graph's linked-tool store current for this step. flowState resolves every linked step
	// at load; here we refresh the one being edited when its link changes (or clear it on unlink), so
	// its tool nodes update without reloading the flow.
	$effect(() => {
		if (!agent) {
			clearLinkedAgentTools(flowPath, moduleId)
			return
		}
		// Publish only once the resource has loaded — don't clobber the load-time tools with [] while
		// the fetch is in flight, which would flicker the tool nodes out of the graph on selection.
		const loaded = linkedResource.current
		if (loaded) {
			// linkedResource types tools loosely; they are the same resource tools the store holds.
			setLinkedAgentTools(flowPath, moduleId, loaded.tools as AgentToolStrict[])
		}
	})

	// Link the step as soon as a saved agent is picked. Linking is rigid, so the step keeps no tools
	// of its own — they come from the resource.
	$effect(() => {
		if (pickerValue && pickerValue !== agent) {
			agent = pickerValue
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

	// The provider is required by the backend (AIAgentArgs.provider is non-optional). Only static
	// transforms are captured into the resource, so a computed/connected provider would be dropped,
	// producing an agent that fails to deserialize on every run — block saving in that case.
	let providerNotStatic = $derived(nonStaticBrainKeys(inputTransforms).includes('provider'))

	// Create or update the `ai_agent` resource at `path` from the step's current brain + tools, then
	// link the step to it.
	async function persist(path: string, description?: string) {
		const dropped = nonStaticBrainKeys(inputTransforms)
		if (providerNotStatic) {
			throw new Error(
				'The provider uses a computed/connected value. Set a static provider before saving — a linked agent needs a provider stored on the resource.'
			)
		}
		if (dropped.length > 0) {
			sendUserToast(
				`Note: ${dropped.join(', ')} use a computed/connected value and won't be saved into the agent`,
				true
			)
		}
		const value = inputTransformsToAgentConfig(inputTransforms, tools)
		const exists = await ResourceService.existsResource({ workspace: ws!, path })
		if (exists) {
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
		editingPath = undefined
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
	async function forkFromResource(): Promise<string | undefined> {
		if (!ws || !agent) {
			return undefined
		}
		const path = agent
		const res = await ResourceService.getResource({ workspace: ws, path })
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
		// The forked step owns the tools directly, with the host bindings folded into each tool's
		// input_transforms, so the linked-only override map no longer applies.
		const forkedTools = cfg.tools ?? []
		for (const tool of forkedTools) {
			const overrides = toolInputs?.[tool.id]
			if (overrides && tool.value?.input_transforms) {
				tool.value.input_transforms = { ...tool.value.input_transforms, ...overrides }
			}
		}
		tools = forkedTools
		toolInputs = {}
		agent = undefined
		pickerValue = undefined
		return path
	}

	// Unlink forks the agent into this step so it can diverge here. It does not write back.
	async function unlink() {
		try {
			const path = await forkFromResource()
			if (path) {
				editingPath = undefined
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
			const path = await forkFromResource()
			if (path) {
				editingPath = path
				sendUserToast(`Editing ${path} — make changes, then Save changes to update it`)
			}
		} catch (e) {
			sendUserToast(`Failed to edit agent: ${e}`, true)
		}
	}
</script>

<div class="px-2 xl:px-4 pt-2">
	{#if agent}
		<div class="rounded-md border border-border bg-surface-secondary px-3 py-2">
			<div class="flex items-center gap-2 text-xs">
				<Bot size={16} class="text-primary shrink-0" />
				<span class="text-secondary shrink-0">Linked to</span>
				<span class="flex min-w-0 items-center gap-1">
					<a class="font-medium truncate" href={`/resources?path=${agent}`} title={agent}>{agent}</a
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
					<Button size="xs2" variant="default" startIcon={{ icon: Pencil }} onclick={editAgent}>
						Edit
					</Button>
					<Button size="xs2" variant="default" startIcon={{ icon: Unlink }} onclick={unlink}>
						Unlink
					</Button>
				</div>
			</div>
			{#if brainParams.length > 0 || inheritedTools.length > 0}
				<dl class="mt-2 flex flex-col gap-1 border-t border-border pt-2">
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
										class="inline-flex items-center rounded border border-border bg-surface px-1.5 py-0.5 text-secondary"
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
		{#if unboundToolInputs > 0}
			<div
				class="mt-1 flex items-start gap-2 rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-2xs text-amber-800 dark:border-amber-800 dark:bg-amber-900/30 dark:text-amber-300"
			>
				<TriangleAlert size={14} class="mt-0.5 shrink-0" />
				<span>
					{unboundToolInputs} tool input{unboundToolInputs > 1 ? 's' : ''}
					{unboundToolInputs > 1 ? 'still reference' : 'still references'} the source flow. Click each
					tool node in the graph to rebind them to this flow before running.
				</span>
			</div>
		{/if}
		{#if !providerOk}
			<div
				class="mt-1 flex items-start gap-2 rounded-md border border-red-300 bg-red-50 px-3 py-2 text-2xs text-red-700 dark:border-red-800 dark:bg-red-900/30 dark:text-red-300"
			>
				<TriangleAlert size={14} class="mt-0.5 shrink-0" />
				<span>
					This agent's model provider{#if providerPath}
						(<span class="font-medium">{providerPath}</span>){/if} isn't accessible in this workspace.
					Unlink to fork the agent, or gain access to the provider resource{#if providerPath}
						at <span class="font-medium">{providerPath}</span>{/if}.
				</span>
			</div>
		{/if}
	{:else if editingPath}
		<div
			class="flex items-center gap-2 rounded-md border border-border bg-surface-secondary px-3 py-2 text-xs"
		>
			<Pencil size={16} class="text-primary shrink-0" />
			<span class="text-secondary">Editing</span>
			<span class="font-medium truncate" title={editingPath}>{editingPath}</span>
			<div class="ml-auto flex items-center gap-1">
				<Button
					size="xs2"
					variant="accent"
					startIcon={{ icon: Save }}
					disabled={saving || providerNotStatic}
					onclick={saveChanges}
				>
					Save changes
				</Button>
				<Button size="xs2" variant="default" onclick={() => (editingPath = undefined)}>
					Cancel
				</Button>
			</div>
		</div>
		<p class="text-2xs text-tertiary mt-1">
			Editing the saved agent. Save changes updates it and re-links this step — the update
			propagates to every flow that links to it. Cancel keeps your edits here as a standalone step
			instead.
		</p>
		{#if providerNotStatic}
			<p class="text-2xs text-red-600 dark:text-red-400 mt-1">
				Set a static provider before saving — a linked agent needs a provider stored on the
				resource.
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
				<input class="text-xs" bind:value={description} placeholder="What this agent does" />
			</label>
			{#if providerNotStatic}
				<p class="text-xs text-red-600 dark:text-red-400">
					Set a static provider before saving — a linked agent needs a provider stored on the
					resource, so a computed/connected provider can't be saved.
				</p>
			{/if}
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				startIcon={{ icon: Save }}
				disabled={!newPath || !!pathError || saving || providerNotStatic}
				onclick={saveAsAgent}
			>
				Save agent
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
