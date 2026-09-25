<script lang="ts">
	import { page } from '$app/state'
	import { FileUp, FormInput, MessageSquare, Pen, Shield, Trash } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import SummaryPathDisplay from '$lib/components/SummaryPathDisplay.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import AgentEditorHost from '$lib/components/flows/content/AgentEditorHost.svelte'
	import AgentConfigModal from '$lib/components/flows/content/AgentConfigModal.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { ResourceService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { isDeployable } from '$lib/utils_deployable'

	/**
	 * The deployed agent, as a flow's or a script's page shows theirs: a way to run it, a chat when
	 * its memory keeps the conversation and the inputs form otherwise, beside what it is.
	 */
	let path = $derived(page.params.path ?? '')
	let ws = $derived($workspaceStore)

	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let agent = $derived(host?.draftHandle())
	let testPane = $derived(host?.testPaneHandle())
	let config = $derived(agent?.state?.args)
	let chatAvailable = $derived(keepsManagedMemory(config?.memory))
	let canEdit = $derived(config != undefined && (agent?.canWrite ?? false) && !$userStore?.operator)
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

	let configModal: AgentConfigModal | undefined = $state(undefined)
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
	<!-- Laid out as `DetailPageHeader` is for flows and scripts, whose trigger and error handler
	     controls an agent does not have. -->
	<div class="border-b">
		<div
			class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center min-h-12 py-2 md:py-0"
		>
			<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
				<div class={twMerge('min-w-0', $userStore?.operator ? 'pl-10' : '')}>
					<SummaryPathDisplay summary={config ? agent?.state?.description : undefined} {path} />
				</div>
			</div>
			<div class="flex gap-1 items-center pr-4">
				<!-- Only an agent that keeps the conversation can chat: without managed memory every
				     message would be answered alone, so it is run from its inputs with nothing to switch. -->
				{#if chatAvailable && !runBlockedReason && testPane?.mode}
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
								size="md"
								value="chat"
								label="Chat"
								icon={MessageSquare}
								tooltip="Chat with the agent: each message runs it, and it remembers the conversation"
								{item}
							/>
							<ToggleButton
								size="md"
								value="form"
								label="Form"
								icon={FormInput}
								tooltip="Run the agent once, on the inputs in a form"
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>
				{/if}
				{#if config && !$userStore?.operator}
					<DropdownV2
						placement="bottom-end"
						size="md"
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
	<div class="flex-1 min-h-0">
		{#key `${ws}:${path}`}
			<AgentEditorHost
				bind:this={host}
				{path}
				workspace={ws}
				enableAi={$copilotInfo.enabled}
				view
				{runBlockedReason}
				onOpenConfig={() => configModal?.open()}
			/>
		{/key}
	</div>
</main>

{#if config}
	<AgentConfigModal bind:this={configModal} {config} toolSchema={(id) => host?.toolSchema(id)} />
{/if}
