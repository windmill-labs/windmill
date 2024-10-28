<script lang="ts">
	import { page } from '$app/stores'
	import {
		FlowService,
		JobService,
		WorkspaceService,
		type Flow,
		type FlowModule,
		type TriggersCount,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import { canWrite, copyToClipboard, defaultIfEmptyString, emptyString } from '$lib/utils'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { Alert, Badge as HeaderBadge, Skeleton } from '$lib/components/common'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import {
		FolderOpen,
		Archive,
		Trash,
		Server,
		Share,
		Badge,
		Loader2,
		GitFork,
		Play,
		History,
		Columns,
		Pen,
		Eye,
		HistoryIcon,
		ClipboardCopy
	} from 'lucide-svelte'

	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import WebhooksPanel from '$lib/components/triggers/WebhooksPanel.svelte'
	import CliHelpBox from '$lib/components/CliHelpBox.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { createAppFromFlow } from '$lib/components/details/createAppFromScript'
	import { importStore } from '$lib/components/apps/store'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import FlowHistory from '$lib/components/flows/FlowHistory.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import Star from '$lib/components/Star.svelte'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { writable } from 'svelte/store'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import WebsocketTriggersPanel from '$lib/components/triggers/WebsocketTriggersPanel.svelte'

	let flow: Flow | undefined
	let can_write = false
	$: path = $page.params.path
	let shareModal: ShareModal
	let deploymentInProgress = false

	let scheduledForStr: string | undefined = undefined
	let invisible_to_owner: boolean | undefined = undefined
	let overrideTag: string | undefined = undefined

	const triggersCount = writable<TriggersCount | undefined>(undefined)

	$: cliCommand = `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`

	let previousPath: string | undefined = undefined
	$: {
		if ($workspaceStore && $userStore && $page.params.path) {
			if (previousPath !== path) {
				previousPath = path
				loadFlow()
				loadTriggersCount()
			}
		}
	}

	async function archiveFlow(): Promise<void> {
		await FlowService.archiveFlowByPath({
			workspace: $workspaceStore!,
			path,
			requestBody: { archived: !flow?.archived }
		})
		loadFlow()
	}

	async function deleteFlow(): Promise<void> {
		await FlowService.deleteFlowByPath({ workspace: $workspaceStore!, path })
		sendUserToast('Flow deleted')
		goto('/')
	}

	let starred: boolean | undefined = undefined

	async function loadTriggersCount() {
		$triggersCount = await FlowService.getTriggersCountOfFlow({
			workspace: $workspaceStore!,
			path
		})
	}

	async function loadFlow(): Promise<void> {
		flow = await FlowService.getFlowByPath({
			workspace: $workspaceStore!,
			path,
			withStarredInfo: true
		})
		starred = flow.starred
		if (!flow.path.startsWith(`u/${$userStore?.username}`) && flow.path.split('/').length > 2) {
			invisible_to_owner = flow.visible_to_runner_only
		}
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}

	let isValid = true
	let loading = false

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner: boolean | undefined,
		overrideTag: string | undefined
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		try {
			let run = await JobService.runFlowByPath({
				workspace: $workspaceStore!,
				path,
				invisibleToOwner,
				requestBody: args,
				scheduledFor,
				tag: overrideTag,
				skipPreprocessor: true
			})
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (e) {
			throw e
		} finally {
			loading = false
		}
	}

	let args: Record<string, any> | undefined = undefined

	let hash = window.location.hash
	if (hash.length > 1) {
		try {
			let searchParams = new URLSearchParams(hash.slice(1))
			let params = [...searchParams.entries()].map(([k, v]) => [k, JSON.parse(v)])
			args = Object.fromEntries(params)
		} catch (e) {
			console.error('Was not able to transform hash as args', e)
		}
	}

	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer
	let runForm: RunForm

	function getMainButtons(flow: Flow | undefined, args: object | undefined) {
		const buttons: any = []

		if (flow && !$userStore?.operator) {
			buttons.push({
				label: 'Fork',
				buttonProps: {
					href: `${base}/flows/add?template=${flow.path}`,
					color: 'light',
					size: 'xs',
					startIcon: GitFork
				}
			})
		}

		if (!flow) {
			return buttons
		}

		buttons.push({
			label: `View runs`,
			buttonProps: {
				href: `${base}/runs/${flow.path}`,
				size: 'xs',
				color: 'light',
				startIcon: Play
			}
		})

		buttons.push({
			label: `History`,
			buttonProps: {
				onClick: () => {
					flowHistory?.open()
				},

				size: 'xs',
				color: 'light',
				startIcon: History
			}
		})

		if (!flow || $userStore?.operator || !can_write) {
			return buttons
		}

		if (!$userStore?.operator) {
			buttons.push({
				label: 'Build App',
				buttonProps: {
					onClick: async () => {
						const app = createAppFromFlow(flow.path, flow.schema)
						$importStore = JSON.parse(JSON.stringify(app))
						await goto('/apps/add?nodraft=true')
					},

					size: 'xs',
					color: 'light',
					startIcon: Columns
				}
			})

			buttons.push({
				label: 'Edit',
				buttonProps: {
					href: `${base}/flows/edit/${path}?nodraft=true`,
					variant: 'contained',
					size: 'xs',
					color: 'dark',
					disabled: !can_write,
					startIcon: Pen
				}
			})
		}
		return buttons
	}

	$: mainButtons = getMainButtons(flow, args)

	let deployUiSettings: WorkspaceDeployUISettings | undefined = undefined

	async function getDeployUiSettings() {
		if (!$enterpriseLicense) {
			deployUiSettings = ALL_DEPLOYABLE
			return
		}
		let settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		deployUiSettings = settings.deploy_ui ?? ALL_DEPLOYABLE
	}
	getDeployUiSettings()

	function getMenuItems(
		flow: Flow | undefined,
		deployUiSettings: WorkspaceDeployUISettings | undefined
	) {
		if (!flow || $userStore?.operator) return []

		const menuItems: any = []

		menuItems.push({
			label: 'Share',
			onclick: () => shareModal.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Share,
			disabled: !can_write
		})

		menuItems.push({
			label: 'Move/Rename',
			onclick: () => moveDrawer.openDrawer(flow?.path ?? '', flow?.summary, 'flow'),
			Icon: FolderOpen
		})

		menuItems.push({
			label: 'Audit logs',
			Icon: Eye,
			onclick: () => {
				goto(`/audit_logs?resource=${flow?.path}`)
			}
		})

		if (isDeployable('flow', flow?.path ?? '', deployUiSettings)) {
			menuItems.push({
				label: 'Deploy to staging/prod',
				onclick: () => deploymentDrawer.openDrawer(flow?.path ?? '', 'flow'),
				Icon: Server
			})
		}

		if (can_write) {
			menuItems.push({
				label: 'Deployments',
				onclick: () => flowHistory?.open(),
				Icon: HistoryIcon
			})
			menuItems.push({
				label: flow.archived ? 'Unarchive' : 'Archive',
				onclick: () => flow?.path && archiveFlow(),
				Icon: Archive,
				color: 'red'
			})
			menuItems.push({
				label: 'Delete',
				onclick: () => flow?.path && deleteFlow(),
				Icon: Trash,
				color: 'red'
			})
		}
		return menuItems
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey || event.metaKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}
	let stepDetail: FlowModule | string | undefined = undefined
	let token = 'TOKEN_TO_CREATE'
	let rightPaneSelected = 'saved_inputs'

	let flowHistory: FlowHistory | undefined = undefined
</script>

<Skeleton
	class="!max-w-7xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>
<svelte:window on:keydown={onKeyDown} />
<DeployWorkspaceDrawer bind:this={deploymentDrawer} />
<ShareModal bind:this={shareModal} />
<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/flows/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadFlow()
	}}
/>
{#if flow}
	<FlowHistory bind:this={flowHistory} path={flow.path} on:historyRestore={loadFlow} />
{/if}

{#if flow}
	<DetailPageLayout
		{triggersCount}
		bind:selected={rightPaneSelected}
		isOperator={$userStore?.operator}
		flow_json={{
			value: flow.value,
			summary: flow.summary,
			description: flow.description,
			schema: flow.schema
		}}
	>
		<svelte:fragment slot="header">
			<DetailPageHeader
				on:seeTriggers={() => {
					rightPaneSelected = 'triggers'
				}}
				{mainButtons}
				menuItems={getMenuItems(flow, deployUiSettings)}
				title={defaultIfEmptyString(flow.summary, flow.path)}
				bind:errorHandlerMuted={flow.ws_error_handler_muted}
				scriptOrFlowPath={flow.path}
				errorHandlerKind="flow"
				tag={flow.tag}
			>
				<svelte:fragment slot="trigger-badges">
					<TriggersBadge
						showOnlyWithCount={true}
						{path}
						newItem={false}
						isFlow
						selected={rightPaneSelected == 'triggers'}
						on:select={() => {
							rightPaneSelected = 'triggers'
						}}
					/>
				</svelte:fragment>
				{#if $workspaceStore}
					<Star
						kind="flow"
						path={flow.path}
						{starred}
						workspace_id={$workspaceStore}
						on:starred={() => {
							starred = !starred
						}}
					/>
				{/if}
				{#if flow?.value?.priority != undefined}
					<div class="hidden md:block">
						<HeaderBadge color="blue" variant="outlined" size="xs">
							{`Priority: ${flow?.value?.priority}`}
						</HeaderBadge>
					</div>
				{/if}
				{#if flow?.value?.concurrent_limit != undefined && flow?.value?.concurrency_time_window_s != undefined}
					<div class="hidden md:block">
						<HeaderBadge color="gray" variant="outlined" size="xs">
							{`Concurrency limit: ${flow?.value?.concurrent_limit} runs every ${flow?.value?.concurrency_time_window_s}s`}
						</HeaderBadge>
					</div>
				{/if}
			</DetailPageHeader>
		</svelte:fragment>
		<svelte:fragment slot="form">
			<div class="flex-col flex h-full justify-between">
				<div class="p-8 w-full max-w-3xl mx-auto gap-2 bg-surface">
					<div class="flex flex-col gap-2 mb-8">
						{#if !emptyString(flow?.description)}
							<GfmMarkdown md={defaultIfEmptyString(flow?.description, 'No description')} />
						{/if}
					</div>

					{#if deploymentInProgress}
						<Badge color="yellow">
							<Loader2 size={12} class="inline animate-spin mr-1" />
							Deployment in progress
						</Badge>
					{/if}

					<RunForm
						bind:scheduledForStr
						bind:invisible_to_owner
						bind:overrideTag
						viewKeybinding
						{loading}
						autofocus
						detailed={false}
						bind:isValid
						runnable={flow}
						runAction={runFlow}
						bind:args
						bind:this={runForm}
					/>
					<div class="py-10" />

					{#if !emptyString(flow.summary)}
						<div class="mb-2">
							<span class="!text-tertiary">{flow.path}</span>
						</div>
					{/if}
					<div class="flex flex-row gap-x-2 flex-wrap items-center">
						<span class="text-sm text-tertiary">
							Edited <TimeAgo date={flow.edited_at ?? ''} /> by {flow.edited_by}
						</span>

						{#if flow.archived}
							<div class="" />
							<Alert type="error" title="Archived">This flow was archived</Alert>
						{/if}
					</div>
				</div>
				<div class="mt-8">
					<FlowGraphViewer
						triggerNode={true}
						download
						{flow}
						overflowAuto
						noSide={true}
						on:select={(e) => {
							if (e.detail) {
								stepDetail = e.detail
								rightPaneSelected = 'flow_step'
							} else {
								stepDetail = undefined
								rightPaneSelected = 'saved_inputs'
							}
						}}
						on:triggerDetail={(e) => {
							rightPaneSelected = 'triggers'
						}}
					/>
				</div>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="save_inputs">
			<SavedInputs
				flowPath={flow?.path}
				{isValid}
				args={args ?? {}}
				on:selected_args={(e) => {
					const nargs = JSON.parse(JSON.stringify(e.detail))
					runForm?.setArgs(nargs)
					args = nargs
				}}
			/>
		</svelte:fragment>
		<svelte:fragment slot="schema">
			<div class="p-1 relative">
				<button
					on:click={() => copyToClipboard(JSON.stringify(flow?.schema, null, 4))}
					class="absolute top-2 right-2"
				>
					<ClipboardCopy size={14} />
				</button>
				<Highlight language={json} code={JSON.stringify(flow?.schema, null, 4)} />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<div class="p-2">
				<WebhooksPanel
					bind:token
					scopes={[`run:flow/${flow?.path}`]}
					path={flow?.path}
					isFlow={true}
					{args}
				/>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="routes">
			<div class="p-2">
				<RoutesPanel path={flow.path ?? ''} isFlow />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="websockets">
			<div class="p-2">
				<WebsocketTriggersPanel path={flow.path ?? ''} isFlow />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="emails">
			<div class="p-2">
				<EmailTriggerPanel
					bind:token
					scopes={[`run:flow/${flow?.path}`]}
					path={flow?.path}
					isFlow={true}
				/>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="schedules">
			<div class="p-2">
				<RunPageSchedules schema={flow.schema} isFlow={true} path={flow.path ?? ''} {can_write} />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="cli">
			<div class="p-2">
				<ClipboardPanel content={cliCommand} />
				<CliHelpBox />
			</div>
		</svelte:fragment>

		<svelte:fragment slot="flow_step">
			{#if stepDetail}
				<FlowGraphViewerStep schema={flow.schema} {stepDetail} />
			{/if}
		</svelte:fragment>
	</DetailPageLayout>
{/if}
