<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Path from '$lib/components/Path.svelte'
	import LightweightResourcePicker from '$lib/components/LightweightResourcePicker.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, Save, Unlink, Pencil, TriangleAlert } from 'lucide-svelte'
	import {
		AGENT_FLOW_LOCAL_KEYS,
		agentConfigToInputTransforms,
		inputTransformsToAgentConfig,
		nonStaticBrainKeys,
		summarizeAgentBrain,
		type AIAgentConfig,
		type AgentTool
	} from '../agentResourceUtils'
	import AgentToolBindings from './AgentToolBindings.svelte'
	import { resource } from 'runed'

	let {
		agent = $bindable(),
		inputTransforms = $bindable(),
		tools = $bindable(),
		toolInputs = $bindable()
	}: {
		agent: string | undefined
		inputTransforms: Record<string, InputTransform>
		tools: AgentTool[]
		toolInputs: Record<string, Record<string, InputTransform>>
	} = $props()

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
		() => ({ ws: $workspaceStore, path: agent }),
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

	// Link the step as soon as a saved agent is picked. Linking is rigid, so the step keeps no tools
	// of its own — they come from the resource.
	$effect(() => {
		if (pickerValue && pickerValue !== agent) {
			agent = pickerValue
			tools = []
			toolInputs = {}
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

	// Create or update the `ai_agent` resource at `path` from the step's current brain + tools, then
	// link the step to it. Updating preserves the resource's existing eval suite (evals are edited
	// in place on the linked step's Evals tab, not here).
	async function persist(path: string, description?: string) {
		const dropped = nonStaticBrainKeys(inputTransforms)
		if (dropped.length > 0) {
			sendUserToast(
				`Note: ${dropped.join(', ')} use a computed/connected value and won't be saved into the agent`,
				true
			)
		}
		const value = inputTransformsToAgentConfig(inputTransforms, tools)
		const exists = await ResourceService.existsResource({ workspace: $workspaceStore!, path })
		if (exists) {
			const cur = await ResourceService.getResource({ workspace: $workspaceStore!, path })
			const curEvals = (cur.value as AIAgentConfig | undefined)?.evals
			if (curEvals) {
				value.evals = curEvals
			}
			await ResourceService.updateResourceValue({
				workspace: $workspaceStore!,
				path,
				requestBody: { value }
			})
		} else {
			await ResourceService.createResource({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					value,
					resource_type: 'ai_agent',
					description: description || 'Reusable AI agent'
				}
			})
		}
		agent = path
		// The brain + tools now live in the resource; a linked step keeps none of its own.
		tools = []
		editingPath = undefined
	}

	async function saveAsAgent() {
		if (!$workspaceStore || pathError || !newPath) {
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
		if (!$workspaceStore || !editingPath) {
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
		if (!$workspaceStore || !agent) {
			return undefined
		}
		const path = agent
		const res = await ResourceService.getResource({ workspace: $workspaceStore, path })
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
		<div
			class="flex items-center gap-2 rounded-md border border-border bg-surface-secondary px-3 py-2 text-xs"
		>
			<Bot size={16} class="text-primary shrink-0" />
			<span class="text-secondary">Linked to</span>
			<a class="font-medium truncate" href={`/resources?path=${agent}`} title={agent}>{agent}</a>
			<div class="ml-auto flex items-center gap-1">
				<Button size="xs2" variant="default" startIcon={{ icon: Pencil }} onclick={editAgent}>
					Edit
				</Button>
				<Button size="xs2" variant="default" startIcon={{ icon: Unlink }} onclick={unlink}>
					Unlink
				</Button>
			</div>
		</div>
		<p class="text-2xs text-tertiary mt-1">
			The configuration below comes from this saved agent and is read-only. Only the message/inputs
			are set in this flow. <span class="font-medium">Edit</span> to change the agent everywhere it's
			used, or Unlink to fork an editable copy into just this step.
		</p>
		{#if brainParams.length > 0 || inheritedTools.length > 0}
			<div class="mt-1 rounded-md border border-border bg-surface-secondary px-3 py-2">
				<div class="text-2xs font-medium text-tertiary uppercase tracking-wide">
					From saved agent · read-only
				</div>
				<dl class="mt-1 flex flex-col gap-1">
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
			</div>
		{/if}
		<AgentToolBindings tools={inheritedTools} bind:toolInputs />
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
					disabled={saving}
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
	{:else}
		<div class="flex items-center gap-2">
			<div class="grow min-w-0">
				<LightweightResourcePicker bind:value={pickerValue} resourceType="ai_agent" />
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
			/>
			<label class="flex flex-col gap-1 text-xs">
				<span class="text-secondary">Description</span>
				<input class="text-xs" bind:value={description} placeholder="What this agent does" />
			</label>
		</div>
		{#snippet actions()}
			<Button
				variant="accent"
				startIcon={{ icon: Save }}
				disabled={!newPath || !!pathError || saving}
				onclick={saveAsAgent}
			>
				Save agent
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
