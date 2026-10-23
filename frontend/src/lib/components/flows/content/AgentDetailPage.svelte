<script lang="ts">
	import { untrack } from 'svelte'
	import { FlaskConical, FormInput, MessageSquare, Pen } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { resource } from 'runed'
	import { goto } from '$lib/navigation'
	import { copilotInfo } from '$lib/aiStore'
	import AgentEvalsModal from '$lib/components/flows/content/AgentEvalsModal.svelte'
	import AgentUsePanel from '$lib/components/flows/content/AgentUsePanel.svelte'
	import { SlackAgentsService } from '$lib/gen'
	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import OnBehalfOfBadge from '$lib/components/details/OnBehalfOfBadge.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import AgentEditorHost from '$lib/components/flows/content/AgentEditorHost.svelte'
	import AgentConfigModal from '$lib/components/flows/content/AgentConfigModal.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import { Alert, Tab, Tabs } from '$lib/components/common'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { agentMenuItems, deleteAgent } from '$lib/components/flows/agentActions'
	import AgentHistory from '$lib/components/flows/AgentHistory.svelte'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { recentItems } from '$lib/components/home/recentItems.svelte'
	import { userStore } from '$lib/stores'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { sendUserToast } from '$lib/toast'
	import { PLAN_MODE_MESSAGES } from '$lib/components/copilot/chat/planModeMessages'
	import type { PendingRun } from '$lib/components/details/pendingRun'

	/**
	 * The deployed agent, as a flow's or a script's page shows theirs: a way to run it, a chat when
	 * its memory keeps the conversation and the inputs form otherwise, beside what it is.
	 */
	interface Props {
		path: string
		workspace: string | undefined
		/** Rendered inside another page (a session tab) rather than as the route: the header stays
		 *  in place instead of taking the page header, and every link goes through `onNavigate`. */
		embedded?: boolean
		/** Whether this page is the one on screen, so its keyboard shortcut is not answered by a
		 *  hidden tab. */
		active?: boolean
		onNavigate?: (url: string) => void | Promise<void>
		/** A chat call waiting on a run of this agent: the form opens on what it proposed, and the
		 *  run started here answers it. */
		pendingRun?: PendingRun
		/** Whether the agent is deployed, once that is known. */
		onLoadState?: (state: 'loaded' | 'not_found') => void
	}

	let {
		path,
		workspace: ws,
		embedded = false,
		active = true,
		onNavigate = (url) => goto(url),
		pendingRun = undefined,
		onLoadState = undefined
	}: Props = $props()

	/** Bumped to read the agent again after it was renamed in place. */
	let reloaded = $state(0)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let agent = $derived(host?.draftHandle())
	let testPane = $derived(host?.testPaneHandle())
	let config = $derived(agent?.state?.args)
	let chatAvailable = $derived(keepsManagedMemory(config?.memory))
	const editRights = useEditRights()
	let canAuthor = $derived(editRights.roleCanAuthor('agent'))
	let canCode = $derived(editRights.capabilities.code)
	let canEdit = $derived(config != undefined && (agent?.canWrite ?? false) && canAuthor)

	// Recorded once the agent is in: a path with nothing deployed never loads one.
	let loaded = $derived(config != undefined)
	$effect(() => {
		if (ws && loaded) {
			recentItems.record(ws, {
				kind: 'agent',
				path,
				summary: agent?.state?.description,
				via: 'view'
			})
		}
	})
	$effect(() => {
		if (!agent || agent.loading) return
		const state = agent.noDeployed ? 'not_found' : loaded ? 'loaded' : undefined
		if (state) untrack(() => onLoadState?.(state))
	})

	// A call waiting on a run is answered from the form, so it opens there even for an agent that
	// would otherwise open on its chat.
	let showedPendingForm = false
	$effect(() => {
		if (!pendingRun || showedPendingForm || !testPane?.mode) return
		showedPendingForm = true
		untrack(() => {
			if (testPane) testPane.mode = 'form'
		})
	})
	let args = $state<Record<string, any>>(
		untrack(() => (pendingRun ? { ...pendingRun.proposedArgs } : {}))
	)

	let configModal: AgentConfigModal | undefined = $state(undefined)
	let shareModal: ShareModal | undefined = $state(undefined)
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state(undefined)
	let history: AgentHistory | undefined = $state(undefined)
	let deleteOpen = $state(false)
	let evalsModal: AgentEvalsModal | undefined = $state(undefined)
	let clientWidth = $state(window.innerWidth)
	let pickedMobileTab = $state('agent')
	// The page is reused across agents, and an agent without managed memory has no Slack tab.
	let mobileTab = $derived(pickedMobileTab === 'slack' && !chatAvailable ? 'api' : pickedMobileTab)
	let wide = $derived(clientWidth >= 1024)

	const slack = resource(
		() => ({ workspace: ws, path }),
		async ({ workspace, path }) =>
			workspace ? await SlackAgentsService.getAgentSlackChannels({ workspace, path }) : undefined
	)
	let runForm: RunForm | undefined = $state(undefined)
	let isValid = $state(true)

	// Cmd/Ctrl+Enter runs the form, as on a script's or a flow's page. The form stays mounted, hidden,
	// while the chat shows, and the chat's composer owns the key then.
	function onKeyDown(event: KeyboardEvent) {
		if (!active) return
		if (event.defaultPrevented || event.key !== 'Enter' || !(event.ctrlKey || event.metaKey)) return
		if (!runForm || testPane?.mode === 'chat' || (!wide && mobileTab !== 'agent')) return
		if (isValid) {
			event.preventDefault()
			runForm.run()
		} else {
			sendUserToast('Please fix errors before running', true)
		}
	}

	// The call a press answers is the one it was pressed under: one settled meanwhile — declined,
	// then another proposed for this same agent — must not be answered with a job it never asked for.
	let pressed: { call: PendingRun | undefined; schema: Record<string, any> } | undefined
	function runStarted(jobId: string, runArgs: Record<string, any>) {
		const at = pressed
		pressed = undefined
		// The call follows the job and shows it in place of this page, so opening the run here as
		// well would leave two surfaces on one job.
		if (at?.call?.adopt(jobId, runArgs, at.schema)) return
		void onNavigate(`/run/${jobId}?workspace=${ws}`)
	}

	async function remove() {
		if (ws && (await deleteAgent(ws, path))) await onNavigate('/?kind=agent')
	}

	// The workspace's, so a path change never waits on it and a menu never outlives its path.
	const deployUiSettings = resource(
		() => ws,
		() => getDeployUiSettings()
	)
	// A role that cannot build agents gets no menu, as on a flow's page.
	let menuItems = $derived(
		config != undefined && canAuthor
			? agentMenuItems({
					path,
					canWrite: canEdit,
					noCode: !canCode,
					wsSpecific: agent?.state?.wsSpecific,
					deployUiSettings: deployUiSettings.current,
					onPermissions: () => shareModal?.openDrawer?.(path, 'resource'),
					onDeploy: () => deploymentDrawer?.openDrawer(path, 'resource'),
					onHistory: () => history?.open(),
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
<AgentHistory bind:this={history} {path} canRestore={canEdit} onRestored={() => reloaded++} />
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

<svelte:window onkeydown={onKeyDown} />

<main class="h-full w-full flex flex-col">
	<!-- `labels`: the pen saves every field it shows in one write, so the labels it shows have to be
	     the ones the resource carries. Handed none, it would offer an empty list, and the first
	     label added there would replace the agent's own. -->
	<DetailPageHeader
		ownsPageHeader={!embedded}
		itemKind="agent"
		summary={config ? agent?.state?.description : undefined}
		labels={agent?.state?.labels}
		{path}
		{menuItems}
		onSaved={canEdit
			? async (newPath) => {
					// A rename moves the page; a summary saved in place has to be read again, since
					// what is shown here came from the load this page keys on.
					if (newPath !== path) await onNavigate(`/agents/get/${newPath}`)
					else reloaded++
				}
			: undefined}
		mainButtons={[
			// Evaluating an agent builds datasets and runs against it: authoring, as editing is, and
			// eval datasets are refused to a role without `code` whatever its flows right.
			...(canEdit && canCode
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
						}
					]
				: []),
			...(canEdit
				? [
						{
							label: 'Edit',
							buttonProps: {
								variant: 'accent',
								unifiedSize: 'md',
								startIcon: Pen,
								href: `/agents/edit/${path}`
							}
						}
					]
				: [])
		]}
	>
		<OnBehalfOfBadge
			onBehalfOf={config?.run_on_behalf_of ? agent?.onBehalfOf : undefined}
			onBehalfOfEmail={undefined}
			kind="agent"
		/>
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
	<!-- Below `lg` the split would squeeze the chat, so the panel's sections become tabs of the
	     page, as on a script's or a flow's page. They cover the agent rather than replacing it:
	     unmounting the host destroys the chat and any turn in flight, so the agent pane keeps one
	     parent at every width. -->
	{#if !wide && ws && config}
		<Tabs
			bind:selected={() => mobileTab, (t) => (pickedMobileTab = t)}
			wrapperClass="flex-none no-scrollbar"
		>
			<Tab value="agent" label={testPane?.mode === 'chat' ? 'Chat' : 'Run form'} />
			<Tab value="api" label="API" />
			{#if chatAvailable}
				<Tab value="slack" label="Slack" />
			{/if}
		</Tabs>
	{/if}
	<div class="relative flex-1 min-h-0" bind:clientWidth>
		<Splitpanes>
			<Pane size={wide ? 65 : 100} minSize={50}>
				<div class="h-full" inert={!wide && mobileTab !== 'agent'}>
					{@render agentPane()}
				</div>
			</Pane>
			{#if wide && ws && config}
				<Pane size={35} minSize={15}>
					{@render usePane(ws)}
				</Pane>
			{/if}
		</Splitpanes>
		{#if !wide && ws && config}
			<div class={['absolute inset-0 z-10 bg-surface', mobileTab === 'agent' && 'hidden']}>
				{@render usePane(ws, mobileTab === 'slack' ? 'slack' : 'api')}
			</div>
		{/if}
	</div>
</main>

{#snippet agentPane()}
	{#if agent?.noDeployed && !onLoadState}
		<div class="p-8 text-center text-sm text-secondary">
			Nothing is deployed at <span class="font-mono">{path}</span> yet.
		</div>
	{/if}
	{#key `${ws}:${path}:${reloaded}`}
		<AgentEditorHost
			bind:this={host}
			{path}
			workspace={ws}
			enableAi={$copilotInfo.enabled}
			view
			onOpenConfig={() => configModal?.open()}
			onRunStarted={embedded || pendingRun ? runStarted : undefined}
		>
			{#snippet viewForm({ schema, run, loading, actions })}
				{#if config && !chatAvailable}
					<Alert type="info" size="xs" title="Managed memory is off" class="mb-4">
						Each run is answered on its own inputs, so this agent has no chat. That suits reusing it
						as a step in a flow.{canEdit
							? ' Turn on managed memory in its configuration to chat with it.'
							: ''}
					</Alert>
				{/if}
				{#if pendingRun}
					<div class="mb-2">
						<InputSelectedBadge inputSelected="pending_run" onReject={() => pendingRun.decline()} />
					</div>
				{/if}
				<RunForm
					runnable={{ schema, path }}
					runAction={(scheduledFor, runArgs) => {
						pressed = { call: pendingRun, schema }
						return run(scheduledFor, runArgs)
					}}
					runRefusal={pendingRun?.planModeActive ? PLAN_MODE_MESSAGES.runFormRefused : undefined}
					schedulable={false}
					syncArgsToUrl={!embedded}
					detailed={false}
					autofocus
					{loading}
					{actions}
					bind:args
					bind:isValid
					bind:this={runForm}
				/>
			{/snippet}
		</AgentEditorHost>
	{/key}
{/snippet}

{#snippet usePane(ws: string, section?: 'api' | 'slack')}
	<AgentUsePanel
		agentPath={path}
		workspace={ws}
		isAdmin={!!($userStore?.is_admin || $userStore?.is_super_admin)}
		chat={chatAvailable}
		slackLoading={slack.loading && !slack.current}
		slackTeamName={slack.current?.slack_team_name}
		botName={slack.current?.bot_name}
		missingScopes={slack.current?.missing_scopes ?? []}
		signingSecretSet={slack.current?.signing_secret_set ?? true}
		runsAs={slack.current?.runs_as}
		anyone={slack.current?.anyone ?? false}
		canShare={slack.current?.can_share ?? false}
		channels={slack.current?.channels ?? []}
		{section}
	/>
{/snippet}

{#if ws}
	<AgentEvalsModal bind:this={evalsModal} agentPath={path} workspace={ws} />
{/if}

{#if config}
	<AgentConfigModal bind:this={configModal} {config} toolSchema={(id) => host?.toolSchema(id)} />
{/if}
