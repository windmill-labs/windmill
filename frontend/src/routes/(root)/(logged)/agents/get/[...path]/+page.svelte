<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { get } from 'svelte/store'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { Boxes, Bot, FileUp, FolderInput, History, Pen, Shield, Trash } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import {
		Alert,
		Badge,
		Button,
		Drawer,
		DrawerContent,
		Tab,
		TabContent,
		Tabs
	} from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import ResourceVersionHistory from '$lib/components/ResourceVersionHistory.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import InheritedLabels from '$lib/components/InheritedLabels.svelte'
	import AgentRunPane from '$lib/components/flows/content/AgentRunPane.svelte'
	import FlowChat from '$lib/components/flows/conversations/FlowChat.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { FormInput, MessageSquare } from 'lucide-svelte'
	import { runFlowPreview } from '$lib/components/flows/utils.svelte'
	import { agentArgsToTransforms } from '$lib/components/flows/linkedAgentDrafts'
	import {
		AGENT_CHAT_BLOCKED_REASON,
		agentChatFlow,
		agentChatReady,
		agentChatStreams,
		type AgentStepShape
	} from '$lib/components/flows/agentChatFlow'
	import { agentWriteCount, markAgentWritten } from '$lib/components/flows/agentEditorStore.svelte'
	import {
		AGENT_BRAIN_LABELS,
		agentEditorRefusal,
		summarizeAgentBrain,
		type AIAgentConfig
	} from '$lib/components/flows/agentResourceUtils'
	import { toolDisplayName, type AgentTool } from '$lib/components/flows/agentToolUtils'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { ResourceService, type Resource } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { canWrite } from '$lib/utils'
	import { isDeployable } from '$lib/utils_deployable'

	/**
	 * The deployed agent, as the flow and script pages show theirs: what it is, and a form to run
	 * it. Everything here reads the deployed resource; the editor page is where the draft lives.
	 */
	let path = $derived(page.params.path ?? '')
	let ws = $derived($workspaceStore)
	let writes = $derived(agentWriteCount(ws, path))

	let agentResource = resource(
		() => ({ ws, path, writes }),
		async ({ ws, path }) => {
			if (!ws || !path) return undefined
			const [r, history, deployUi] = await Promise.all([
				ResourceService.getResource({ workspace: ws, path }),
				ResourceService.getResourceHistory({ workspace: ws, path }).catch(() => undefined),
				getDeployUiSettings()
			])
			return {
				resource: r,
				version: history?.versions?.[0]?.version,
				deployable: isDeployable('resource', path, deployUi)
			}
		}
	)
	let loaded = $derived(agentResource.current)
	let agent = $derived(loaded?.resource as Resource | undefined)
	let refusal = $derived(agent ? agentEditorRefusal(path, agent.resource_type) : undefined)
	let config = $derived((agent?.value ?? {}) as AIAgentConfig)
	let brain = $derived(summarizeAgentBrain(config))
	let tools = $derived(Array.isArray(config.tools) ? (config.tools as AgentTool[]) : [])
	let systemPrompt = $derived(
		typeof config.system_prompt === 'string' ? config.system_prompt : undefined
	)
	let can_write = $derived(
		agent ? canWrite(agent.path, agent.extra_perms ?? {}, get(userStore) ?? undefined) : false
	)
	let loadError = $derived(agentResource.error ? String(agentResource.error) : undefined)

	let rightTab = $state<'agent' | 'export'>('agent')

	/** Chat first, as in the editor; the form stays for a run with explicit inputs. */
	let leftPane = $state<'chat' | 'form'>('chat')
	// The deployed config in the shape a flow step carries it, which is what the chat runs.
	let chatStep = $derived<AgentStepShape>({
		input_transforms: agentArgsToTransforms(config) as AgentStepShape['input_transforms'],
		tools
	})
	let chatReady = $derived(agentChatReady(chatStep))
	let chatBlockedReason = $derived(chatReady ? undefined : AGENT_CHAT_BLOCKED_REASON)
	let useStreaming = $derived(agentChatStreams(chatStep))
	let chatFlowShape = $derived(agentChatFlow(chatStep))

	// A turn previews the deployed config under the agent's path. There is no deployed run for an
	// agent on the server, so these conversations are filed as the preview files them: test ones,
	// shared with the editor's list.
	async function runChatTurn(
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	): Promise<string | undefined> {
		return await runFlowPreview(
			{ user_message: userMessage, ...(additionalInputs ?? {}) },
			agentChatFlow(chatStep),
			path,
			undefined,
			conversationId,
			undefined,
			ws
		)
	}

	/** What kind of tool a roster entry is: an MCP server, a web search, a script, a flow... */
	function toolKind(tool: AgentTool): string {
		const value = tool?.value as Record<string, any> | undefined
		return value?.tool_type ?? value?.type ?? 'tool'
	}

	let shareModal: ShareModal | undefined = $state(undefined)
	let moveDrawer: MoveDrawer | undefined = $state(undefined)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)
	let versionDrawer: Drawer | undefined = $state(undefined)
	let deleteOpen = $state(false)

	async function deleteAgent() {
		if (!ws) return
		try {
			await ResourceService.deleteResource({ workspace: ws, path })
			sendUserToast(`Deleted agent ${path}`)
			await goto(`${base}/?kind=agent`)
		} catch (err) {
			sendUserToast(`Could not delete agent ${path}: ${err}`, true)
		}
	}

	let menuItems = $derived.by(() => {
		if (!agent || $userStore?.operator) return []
		return [
			{
				displayName: 'Permissions',
				icon: Shield,
				disabled: !can_write,
				action: () => shareModal?.openDrawer?.(path, 'resource')
			},
			{
				displayName: 'Move/Rename',
				icon: FolderInput,
				disabled: !can_write,
				action: () => moveDrawer?.openDrawer(path, agent?.description, 'resource')
			},
			{
				displayName: 'Version history',
				icon: History,
				action: () => versionDrawer?.openDrawer()
			},
			{
				displayName: 'Open in resources',
				icon: Boxes,
				href: `${base}/resources#/resource/${path}`
			},
			...(loaded?.deployable && !agent.ws_specific
				? [
						{
							displayName: 'Deploy to prod/staging',
							icon: FileUp,
							action: () => deploymentDrawer?.openDrawer(path, 'resource')
						}
					]
				: []),
			{
				displayName: 'Delete',
				icon: Trash,
				type: 'delete' as const,
				disabled: !can_write,
				action: () => (deleteOpen = true)
			}
		]
	})
</script>

<ShareModal bind:this={shareModal} />
<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto(`${base}/agents/get/${e.detail}?workspace=${ws}`)
	}}
/>
<ConfirmationModal
	open={deleteOpen}
	title="Delete agent"
	confirmationText="Delete"
	trashbin
	on:canceled={() => (deleteOpen = false)}
	on:confirmed={() => {
		deleteOpen = false
		deleteAgent()
	}}
>
	<span>Every flow linking {path} will fail at its agent step once it is gone.</span>
</ConfirmationModal>
<Drawer bind:this={versionDrawer} size="1200px">
	<DrawerContent title="Version history" on:close={() => versionDrawer?.closeDrawer()} noPadding>
		<ResourceVersionHistory
			{path}
			workspace={ws}
			canRestore={can_write}
			onRestore={() => {
				versionDrawer?.closeDrawer()
				if (ws) markAgentWritten(ws, path)
			}}
		/>
	</DrawerContent>
</Drawer>

<main class="h-screen w-full flex flex-col">
	<div class="border-b">
		<div
			class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-12"
		>
			<div class="grow px-4 inline-flex items-center gap-3 min-w-0">
				<Bot size={20} class="text-violet-500 shrink-0" />
				<div class="min-w-0 flex flex-col">
					<span class="text-sm font-semibold text-emphasis truncate">
						{agent?.description || path}
					</span>
					{#if agent?.description}
						<span class="text-2xs text-tertiary truncate">{path}</span>
					{/if}
				</div>
				{#if loaded?.version != undefined}
					<Badge color="gray" class="shrink-0" title="The version runs are recorded against">
						v{loaded.version}
					</Badge>
				{/if}
				{#if agent?.labels?.length}
					<div class="hidden md:flex items-center gap-0.5">
						{#each agent.labels as label (label)}
							<Badge color="blue" small class="px-1">{label}</Badge>
						{/each}
					</div>
				{/if}
				<InheritedLabels labels={agent?.inherited_labels} />
			</div>
			{#if agent && !refusal}
				<ToggleButtonGroup
					selected={leftPane}
					onSelected={(v) => (leftPane = v as 'chat' | 'form')}
					noWFull
				>
					{#snippet children({ item })}
						<ToggleButton
							value="chat"
							label="Chat"
							icon={MessageSquare}
							size="md"
							disabled={!chatReady}
							tooltip={chatBlockedReason}
							{item}
						/>
						<ToggleButton value="form" label="Form" icon={FormInput} size="md" {item} />
					{/snippet}
				</ToggleButtonGroup>
				<div class="grow"></div>
			{/if}
			<div class="flex gap-1 items-center pr-4">
				{#if menuItems.length > 0}
					<DropdownV2 items={menuItems} placement="bottom-end" size="md" />
				{/if}
				{#if can_write && !$userStore?.operator}
					<Button
						variant="accent"
						unifiedSize="md"
						startIcon={{ icon: Pen }}
						href="{base}/agents/edit/{path}"
					>
						Edit
					</Button>
				{/if}
			</div>
		</div>
	</div>

	{#if loadError}
		<div class="p-6 max-w-lg">
			<Alert type="error" title="Could not load agent">{loadError}</Alert>
		</div>
	{:else if refusal}
		<div class="p-6 max-w-lg">
			<Alert type="error" title="Not an agent">{refusal}</Alert>
		</div>
	{:else if agent}
		<div class="grow min-h-0 w-full">
			<Splitpanes>
				<Pane size={65} minSize={40}>
					<div class="h-full flex flex-col min-h-0">
						<!-- In chat the description belongs to the chat, which shows it under the empty
						     transcript and in the sidebar. -->
						{#if agent.description && leftPane === 'form'}
							<div class="px-4 pt-4 shrink-0">
								<div class="p-4 rounded-md bg-surface-secondary">
									<GfmMarkdown md={agent.description} noPadding />
								</div>
							</div>
						{/if}
						<div class="grow min-h-0 flex flex-col">
							{#if leftPane === 'chat' && chatReady}
								<FlowChat
									{useStreaming}
									onRunFlow={runChatTurn}
									conversationKind="test"
									frame="none"
									{path}
									identity={path}
									description={agent.description}
									inputSchema={chatFlowShape.schema}
									flowModules={chatFlowShape.value.modules}
								/>
							{:else if leftPane === 'chat'}
								<div class="flex-1 flex items-center justify-center p-6 text-xs text-tertiary">
									{chatBlockedReason}
								</div>
							{:else}
								<!-- Keyed on the write count: the pane builds its step once from the config
								     it is given, so a restore or a deploy from another tab gets a fresh one. -->
								{#key `${ws}:${path}:${writes}`}
									<AgentRunPane {path} workspace={ws} {config} />
								{/key}
							{/if}
						</div>
					</div>
				</Pane>
				<Pane size={35} minSize={20}>
					<div class="flex flex-col h-full">
						<Tabs bind:selected={rightTab} wrapperClass="flex-none w-full">
							<Tab value="agent" label="Agent" />
							<Tab value="export" label="Export" />
							{#snippet content()}
								<div class="min-h-0 grow overflow-auto">
									<TabContent value="agent" class="p-4 flex flex-col gap-6">
										<section class="flex flex-col gap-2">
											<h3 class="text-xs font-semibold uppercase text-tertiary">Model</h3>
											<dl class="flex flex-col gap-1">
												{#each brain.filter((p) => p.label !== AGENT_BRAIN_LABELS['system_prompt']) as param (param.label)}
													<div class="flex items-baseline gap-2 text-xs">
														<dt class="text-tertiary shrink-0 w-32">{param.label}</dt>
														<dd class="text-secondary truncate" title={param.value}
															>{param.value}</dd
														>
													</div>
												{/each}
											</dl>
										</section>
										<section class="flex flex-col gap-2">
											<h3 class="text-xs font-semibold uppercase text-tertiary">System message</h3>
											{#if systemPrompt}
												<pre
													class="text-xs text-secondary whitespace-pre-wrap rounded-md border border-light bg-surface-tertiary p-3 max-h-80 overflow-auto"
													>{systemPrompt}</pre
												>
											{:else}
												<span class="text-xs text-tertiary">None</span>
											{/if}
										</section>
										<section class="flex flex-col gap-2">
											<h3 class="text-xs font-semibold uppercase text-tertiary">
												Tools ({tools.length})
											</h3>
											{#if tools.length === 0}
												<span class="text-xs text-tertiary">No tools.</span>
											{:else}
												<ul class="flex flex-col gap-1">
													{#each tools as tool (tool.id)}
														<li class="flex items-center gap-2 text-xs">
															<span class="text-secondary truncate">
																{toolDisplayName(tool) ?? tool.id}
															</span>
															<Badge color="gray" small>{toolKind(tool)}</Badge>
														</li>
													{/each}
												</ul>
											{/if}
										</section>
									</TabContent>
									<TabContent value="export" class="p-2">
										<HighlightCode
											language="json"
											code={JSON.stringify(agent.value ?? {}, null, 2)}
										/>
									</TabContent>
								</div>
							{/snippet}
						</Tabs>
					</div>
				</Pane>
			</Splitpanes>
		</div>
	{:else}
		<div class="p-6 text-xs text-tertiary">Loading agent...</div>
	{/if}
</main>
