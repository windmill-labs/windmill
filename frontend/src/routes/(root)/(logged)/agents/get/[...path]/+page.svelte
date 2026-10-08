<script lang="ts">
	import { page } from '$app/state'
	import { FlaskConical, FormInput, MessageSquare, Pen } from 'lucide-svelte'
	import { resource } from 'runed'
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import AgentEvalsModal from '$lib/components/flows/content/AgentEvalsModal.svelte'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import AgentEditorHost from '$lib/components/flows/content/AgentEditorHost.svelte'
	import AgentConfigModal from '$lib/components/flows/content/AgentConfigModal.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { agentMenuItems, deleteAgent } from '$lib/components/flows/agentActions'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { userStore, workspaceStore } from '$lib/stores'

	/**
	 * The deployed agent, as a flow's or a script's page shows theirs: a way to run it, a chat when
	 * its memory keeps the conversation and the inputs form otherwise, beside what it is.
	 */
	let path = $derived(page.params.path ?? '')
	let ws = $derived($workspaceStore)

	/** Bumped to read the agent again after it was renamed in place. */
	let reloaded = $state(0)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let agent = $derived(host?.draftHandle())
	let testPane = $derived(host?.testPaneHandle())
	let config = $derived(agent?.state?.args)
	let chatAvailable = $derived(keepsManagedMemory(config?.memory))
	let canEdit = $derived(config != undefined && (agent?.canWrite ?? false) && !$userStore?.operator)

	let configModal: AgentConfigModal | undefined = $state(undefined)
	let shareModal: ShareModal | undefined = $state(undefined)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)
	let deleteOpen = $state(false)
	let evalsModal: AgentEvalsModal | undefined = $state(undefined)

	async function remove() {
		if (ws && (await deleteAgent(ws, path))) await goto(`${base}/?kind=agent`)
	}

	// The workspace's, so a path change never waits on it and a menu never outlives its path.
	const deployUiSettings = resource(
		() => ws,
		() => getDeployUiSettings()
	)
	// Operators get no menu, as on a flow's or a script's page.
	let menuItems = $derived(
		config != undefined && !$userStore?.operator
			? agentMenuItems({
					path,
					canWrite: canEdit,
					wsSpecific: agent?.state?.wsSpecific,
					deployUiSettings: deployUiSettings.current,
					onPermissions: () => shareModal?.openDrawer?.(path, 'resource'),
					onDeploy: () => deploymentDrawer?.openDrawer(path, 'resource'),
					onDelete: () => (deleteOpen = true)
				}).map((item) => ({
					label: item.displayName,
					Icon: item.icon,
					onclick: (e: MouseEvent) => item.action?.(e),
					disabled: item.disabled,
					color: item.type === 'delete' ? ('red' as const) : undefined
				}))
			: []
	)
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
		remove()
	}}
>
	<span>Every flow that links {path} will fail at its agent step once it is deleted.</span>
</ConfirmationModal>

<main class="h-full w-full flex flex-col">
	<!-- `labels`: the pen saves every field it shows in one write, so the labels it shows have to be
	     the ones the resource carries. Handed none, it would offer an empty list, and the first
	     label added there would replace the agent's own. -->
	<DetailPageHeader
		ownsPageHeader
		itemKind="agent"
		summary={config ? agent?.state?.description : undefined}
		labels={agent?.state?.labels}
		{path}
		{menuItems}
		onSaved={canEdit
			? async (newPath) => {
					// A rename moves the page; a summary saved in place has to be read again, since
					// what is shown here came from the load this page keys on.
					if (newPath !== path) await goto(`${base}/agents/get/${newPath}`)
					else reloaded++
				}
			: undefined}
		mainButtons={[
			// Evaluating an agent builds datasets and runs against it: authoring, as editing is.
			...(canEdit
				? [
						{
							label: 'Evals',
							buttonProps: {
								variant: 'default',
								unifiedSize: 'md',
								startIcon: FlaskConical,
								title: 'Run this agent against a dataset of cases',
								onClick: () => evalsModal?.openModal()
							}
						},
						{
							label: 'Edit',
							buttonProps: {
								variant: 'accent',
								unifiedSize: 'md',
								startIcon: Pen,
								href: `${base}/agents/edit/${path}`
							}
						}
					]
				: [])
		]}
	>
		{#snippet leading_actions()}
			<!-- Only an agent that keeps the conversation can chat: without managed memory every
			     message would be answered alone, so it is run from its inputs with nothing to switch. -->
			{#if chatAvailable && testPane?.mode}
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
		{/snippet}
	</DetailPageHeader>
	<div class="flex-1 min-h-0">
		{#key `${ws}:${path}:${reloaded}`}
			<AgentEditorHost
				bind:this={host}
				{path}
				workspace={ws}
				enableAi={$copilotInfo.enabled}
				view
				onOpenConfig={() => configModal?.open()}
			>
				{#snippet viewForm({ schema, run, loading, actions })}
					<RunForm
						runnable={{ schema, path }}
						runAction={run}
						schedulable={false}
						detailed={false}
						autofocus
						{loading}
						{actions}
					/>
				{/snippet}
			</AgentEditorHost>
		{/key}
	</div>
</main>

{#if ws}
	<AgentEvalsModal bind:this={evalsModal} agentPath={path} workspace={ws} />
{/if}

{#if config}
	<AgentConfigModal bind:this={configModal} {config} toolSchema={(id) => host?.toolSchema(id)} />
{/if}
