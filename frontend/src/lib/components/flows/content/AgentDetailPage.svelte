<script lang="ts">
	import { untrack } from 'svelte'
	import { FlaskConical, FormInput, MessageSquare, Pen } from 'lucide-svelte'
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
	import BuildChatAppCard from '$lib/components/details/BuildChatAppCard.svelte'
	import { createChatRawApp } from '$lib/components/details/createRawAppFromScript'
	import AgentConfigModal from '$lib/components/flows/content/AgentConfigModal.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import { Alert, Tab, Tabs } from '$lib/components/common'
	import { keepsManagedMemory } from '$lib/components/flows/agentFormFields'
	import { agentMenuItems, deleteAgent } from '$lib/components/flows/agentActions'
	import AgentHistory from '$lib/components/flows/AgentHistory.svelte'
	import { getDeployUiSettings } from '$lib/components/home/deploy_ui'
	import { recentItems } from '$lib/components/home/recentItems.svelte'
	import { useEditRights } from '$lib/operatorWriteRights'
	import { sendUserToast } from '$lib/toast'
	import { PLAN_MODE_MESSAGES } from '$lib/components/copilot/chat/planModeMessages'
	import type { PendingRun } from '$lib/components/details/pendingRun'
	import {
		setOperatingWorkspace,
		useOperatingUser
	} from '$lib/components/operatingWorkspace.svelte'

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
		/** The deployed agent's summary, for a host that names the page after it. */
		onSummary?: (summary: string | undefined) => void
	}

	let {
		path,
		workspace: ws,
		embedded = false,
		active = true,
		onNavigate = (url) => goto(url),
		pendingRun = undefined,
		onLoadState = undefined,
		onSummary = undefined
	}: Props = $props()

	// Every workspace-scoped call below and in the drawers this page opens (the summary's save,
	// permissions, history) targets `workspace`, not the nav store: a session shows the agent of its
	// own workspace, which a fork makes a different one.
	setOperatingWorkspace(() => ws)

	// The user as this workspace knows them: in a fork, the navigation user is the parent's.
	const operatingUser = useOperatingUser()

	/** Bumped to read the agent again after it was renamed in place. */
	let reloaded = $state(0)
	let host = $state<ReturnType<typeof AgentEditorHost> | undefined>(undefined)
	let agent = $derived(host?.draftHandle())
	let testPane = $derived(host?.testPaneHandle())
	let config = $derived(agent?.state?.args)
	let chatAvailable = $derived(keepsManagedMemory(config?.memory))
	const editRights = useEditRights(() => ws)
	let canAuthor = $derived(editRights.roleCanAuthor('agent'))
	let canCode = $derived(editRights.capabilities.code)

	async function buildChatApp() {
		const app = createChatRawApp({ path, runType: 'agent' })
		// The raw app editor reads the payload from sessionStorage on mount.
		sessionStorage.setItem('rawAppImport', JSON.stringify(app))
		await goto('/apps_raw/add')
	}
	let canEdit = $derived(config != undefined && (agent?.canWrite ?? false) && canAuthor)

	// Recorded once the agent is in: a path with nothing deployed never loads one. Not from a
	// session's tab, as a script's page in one records nothing either.
	let loaded = $derived(config != undefined)
	$effect(() => {
		if (ws && loaded && !embedded) {
			recentItems.record(ws, {
				kind: 'agent',
				path,
				summary: agent?.state?.description,
				via: 'view'
			})
		}
	})
	// Reported once loaded: a host takes the first report as the name for good, an empty one included.
	$effect(() => {
		if (!config) return
		const summary = agent?.state?.description?.trim() || undefined
		untrack(() => onSummary?.(summary))
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
	let pickedTab = $state<'agent' | 'api' | 'slack'>('agent')
	// The page is reused across agents, and an agent without managed memory has no Slack tab.
	let tab = $derived(
		pickedTab !== 'agent' && pickedTab !== 'api' && !chatAvailable ? 'api' : pickedTab
	)

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
		if (!runForm || testPane?.mode === 'chat' || tab !== 'agent') return
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
		if (!ws || !(await deleteAgent(ws, path))) return
		// Embedded, the host owns the tab: read again so it shows that nothing is deployed here,
		// rather than opening Home beside a tab still showing the deleted agent.
		if (embedded) reloaded++
		else await onNavigate('/?kind=agent')
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
					onDelete: () => (deleteOpen = true),
					onBuildChatApp:
						chatAvailable && editRights.roleCanAuthor('raw_app') ? buildChatApp : undefined
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
	<!-- Tabs at every width, as a chat-mode flow's page has them. They cover the agent rather than
	     replacing it: unmounting the host destroys the chat and any turn in flight. -->
	{#if ws && config}
		<Tabs bind:selected={() => tab, (t) => (pickedTab = t)} wrapperClass="flex-none no-scrollbar">
			<Tab value="agent" label={testPane?.mode === 'chat' ? 'Chat' : 'Run form'} />
			<Tab value="api" label="API" />
			{#if chatAvailable}
				<Tab value="slack" label="Slack" />
			{/if}
		</Tabs>
	{/if}
	<div class="relative flex-1 min-h-0">
		<div class="h-full" inert={tab !== 'agent'}>
			{@render agentPane()}
		</div>
		{#if ws && config}
			<div class={['absolute inset-0 z-10 bg-surface', tab === 'agent' && 'hidden']}>
				{@render usePane(tab === 'agent' ? 'api' : tab)}
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
			{#snippet chatSidebarFooter({ expanded })}
				{#if ws}
					<BuildChatAppCard kind="agent" {path} workspace={ws} {expanded} />
				{/if}
			{/snippet}
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

{#snippet usePane(section: 'api' | 'slack')}
	<AgentUsePanel
		agentPath={path}
		isAdmin={!!(operatingUser.current?.is_admin || operatingUser.current?.is_super_admin)}
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
