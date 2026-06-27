<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Path from '$lib/components/Path.svelte'
	import LightweightResourcePicker from '$lib/components/LightweightResourcePicker.svelte'
	import { ResourceService, type InputTransform } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Bot, Save, Unlink, Pencil } from 'lucide-svelte'
	import {
		AGENT_FLOW_LOCAL_KEYS,
		agentConfigToInputTransforms,
		inputTransformsToAgentConfig,
		nonStaticBrainKeys,
		type AIAgentConfig,
		type AgentTool
	} from '../agentResourceUtils'

	let {
		agent = $bindable(),
		inputTransforms = $bindable(),
		tools = $bindable(),
		onEdit
	}: {
		agent: string | undefined
		inputTransforms: Record<string, InputTransform>
		tools: AgentTool[]
		onEdit?: () => void
	} = $props()

	let saveDrawer: Drawer | undefined = $state()
	let newPath = $state('')
	let pathError = $state('')
	let description = $state('')
	let saving = $state(false)
	let pickerValue: string | undefined = $state(undefined)

	// Link the step as soon as a saved agent is picked.
	$effect(() => {
		if (pickerValue && pickerValue !== agent) {
			agent = pickerValue
		}
	})

	function openSave() {
		newPath = ''
		pathError = ''
		description = ''
		saveDrawer?.openDrawer()
	}

	async function saveAsAgent() {
		if (!$workspaceStore || pathError || !newPath) {
			return
		}
		saving = true
		try {
			// Only static brain values are portable into a reusable agent; flag any that won't carry over.
			const dropped = nonStaticBrainKeys(inputTransforms)
			if (dropped.length > 0) {
				sendUserToast(
					`Note: ${dropped.join(', ')} use a computed/connected value and won't be saved into the agent`,
					true
				)
			}
			const value = inputTransformsToAgentConfig(inputTransforms, tools)
			await ResourceService.createResource({
				workspace: $workspaceStore,
				requestBody: {
					path: newPath,
					value,
					resource_type: 'ai_agent',
					description: description || 'Reusable AI agent'
				}
			})
			agent = newPath
			saveDrawer?.closeDrawer()
			sendUserToast(`Saved reusable agent ${newPath}`)
		} catch (e) {
			sendUserToast(`Failed to save agent: ${e}`, true)
		} finally {
			saving = false
		}
	}

	async function unlink() {
		if (!$workspaceStore || !agent) {
			return
		}
		try {
			const res = await ResourceService.getResource({ workspace: $workspaceStore, path: agent })
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
			tools = cfg.tools ?? []
			agent = undefined
			pickerValue = undefined
			sendUserToast('Unlinked agent — config copied into the step')
		} catch (e) {
			sendUserToast(`Failed to unlink agent: ${e}`, true)
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
				{#if onEdit}
					<Button size="xs2" variant="default" startIcon={{ icon: Pencil }} onclick={onEdit}>
						Edit
					</Button>
				{/if}
				<Button size="xs2" variant="default" startIcon={{ icon: Unlink }} onclick={unlink}>
					Unlink
				</Button>
			</div>
		</div>
		<p class="text-2xs text-tertiary mt-1">
			The agent brain and tools come from this resource. Only the message/inputs below are set in
			this flow.
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
				<input
					class="text-xs"
					bind:value={description}
					placeholder="What this agent does"
				/>
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
