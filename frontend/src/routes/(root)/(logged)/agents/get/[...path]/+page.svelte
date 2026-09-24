<script lang="ts">
	import { page } from '$app/state'
	import { Bot, FileUp, FormInput, MessageSquare, Pen, Shield, Trash } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import AgentEditorHost from '$lib/components/flows/content/AgentEditorHost.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { ResourceService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isDeployable } from '$lib/utils_deployable'

	/**
	 * The deployed agent, as a flow's or a script's page shows theirs: what it is, and a way to run
	 * it. A chat when its memory keeps the conversation, the inputs form otherwise.
	 */
	let path = $derived(page.params.path ?? '')
	let ws = $derived($workspaceStore)

	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let toolId = $state<string | undefined>(undefined)
	let agent = $derived(host?.draftHandle())
	let testPane = $derived(host?.testPaneHandle())
	let loaded = $derived(agent?.state != undefined)
	let chatAvailable = $derived(keepsManagedMemory(agent?.state?.args?.memory))
	let canEdit = $derived(loaded && (agent?.canWrite ?? false) && !$userStore?.operator)
	// An agent runs as a flow preview, and the server refuses a preview to an operator, and to a
	// non-admin under another user's namespace (`require_path_read_access_for_preview`).
	let runBlockedReason = $derived(
		$userStore?.operator
			? 'Operators cannot run agents yet.'
			: !$userStore?.is_admin &&
				  path.startsWith('u/') &&
				  !path.startsWith(`u/${$userStore?.username}/`)
				? `Only its owner and workspace admins can run an agent under ${path.split('/').slice(0, 2).join('/')}.`
				: undefined
	)

	let shareModal: ShareModal | undefined = $state(undefined)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)
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
</script>

<ShareModal bind:this={shareModal} />
<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ConfirmationModal
	open={deleteOpen}
	title="Delete agent"
	confirmationText="Delete"
	on:canceled={() => (deleteOpen = false)}
	on:confirmed={() => {
		deleteOpen = false
		deleteAgent()
	}}
>
	<span>Every flow that links {path} will fail at its agent step once it is deleted.</span>
</ConfirmationModal>

<main class="h-screen w-full flex flex-col">
	<div class="flex items-center gap-3 px-4 py-2 border-b shrink-0 min-h-12">
		<Bot size={20} class="text-violet-500 shrink-0" />
		<div class="min-w-0 flex flex-col">
			<span class="text-sm font-semibold text-emphasis truncate">
				{agent?.state?.description || path}
			</span>
			{#if agent?.state?.description}
				<span class="text-2xs text-tertiary truncate">{path}</span>
			{/if}
		</div>
		<div class="grow"></div>
		<!-- Only an agent that keeps the conversation can chat: without managed memory every message
		     would be answered alone, so it is run from its inputs and nothing is offered to switch. -->
		{#if loaded && chatAvailable && !runBlockedReason && testPane?.mode}
			<ToggleButtonGroup
				bind:selected={
					() => testPane?.mode,
					(mode) => {
						if (testPane) testPane.mode = mode
					}
				}
				noWFull
			>
				{#snippet children({ item })}
					<ToggleButton
						size="sm"
						value="chat"
						label="Chat"
						icon={MessageSquare}
						tooltip="Chat with the agent: each message runs it, and it remembers the conversation"
						{item}
					/>
					<ToggleButton
						size="sm"
						value="form"
						label="Form"
						icon={FormInput}
						tooltip="Run the agent once, on the inputs in a form"
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
			<div class="grow"></div>
		{/if}
		{#if loaded && !$userStore?.operator}
			<DropdownV2
				items={async () => [
					{
						displayName: 'Permissions',
						icon: Shield,
						disabled: !canEdit,
						action: () => shareModal?.openDrawer?.(path, 'resource')
					},
					...(!agent?.state?.wsSpecific &&
					isDeployable('resource', path, await getDeployUiSettings())
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
						disabled: !canEdit,
						action: () => (deleteOpen = true)
					}
				]}
			/>
		{/if}
		{#if canEdit}
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: Pen }}
				href="{base}/agents/edit/{path}"
			>
				Edit
			</Button>
		{/if}
	</div>
	<div class="flex-1 min-h-0">
		{#key `${ws}:${path}`}
			<AgentEditorHost
				bind:this={host}
				{path}
				workspace={ws}
				enableAi={$copilotInfo.enabled}
				{toolId}
				onSelectTool={(id) => (toolId = id)}
				view
				{runBlockedReason}
			/>
		{/key}
	</div>
</main>
