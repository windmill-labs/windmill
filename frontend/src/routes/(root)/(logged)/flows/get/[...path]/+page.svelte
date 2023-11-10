<script lang="ts">
	import { page } from '$app/stores'
	import { FlowService, JobService, type Flow, type FlowModule } from '$lib/gen'
	import { canWrite, defaultIfEmptyString, emptyString, encodeState } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import { goto } from '$app/navigation'
	import { Alert, Badge as HeaderBadge, Skeleton } from '$lib/components/common'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { runFormStore, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import Urlize from '$lib/components/Urlize.svelte'
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
		History,
		Columns,
		Pen
	} from 'lucide-svelte'

	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import CliHelpBox from '$lib/components/CliHelpBox.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { createAppFromFlow } from '$lib/components/details/createAppFromScript'
	import { importStore } from '$lib/components/apps/store'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	let flow: Flow | undefined
	let can_write = false
	let path = $page.params.path
	let shareModal: ShareModal
	let deploymentInProgress = false

	$: cliCommand = `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`

	$: {
		if ($workspaceStore && $userStore) {
			loadFlow()
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

	async function loadFlow(): Promise<void> {
		flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}

	$: urlAsync = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run/f/${flow?.path}`
	$: urlSync = `${$page.url.origin}/api/w/${$workspaceStore}/jobs/run_wait_result/f/${flow?.path}`

	let isValid = true
	let loading = false

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		let run = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path,
			invisibleToOwner,
			requestBody: args,
			scheduledFor
		})
		await goto('/run/' + run + '?workspace=' + $workspaceStore)
	}

	let args = undefined

	if ($runFormStore) {
		args = $runFormStore
		$runFormStore = undefined
	}

	let moveDrawer: MoveDrawer
	let deploymentDrawer: DeployWorkspaceDrawer
	let runForm: RunForm

	function getMainButtons(flow: Flow | undefined, args: object | undefined) {
		const buttons: any = []

		if (flow) {
			buttons.push({
				label: 'Fork',
				buttonProps: {
					href: `/flows/add?template=${flow.path}`,
					variant: 'border',
					color: 'light',
					size: 'xs',
					startIcon: GitFork
				}
			})
		}

		if (!flow || $userStore?.operator || !can_write) {
			return buttons
		}

		buttons.push({
			label: `View runs`,
			buttonProps: {
				href: `/runs/${flow.path}`,
				size: 'xs',
				color: 'light',
				startIcon: History
			}
		})

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
					href: `/flows/edit/${path}?nodraft=true&args=${encodeState(args)}`,
					variant: 'contained',
					size: 'sm',
					color: 'dark',
					disabled: !can_write,
					startIcon: Pen
				}
			})
		}
		return buttons
	}

	$: mainButtons = getMainButtons(flow, args)

	function getMenuItems(flow: Flow | undefined) {
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
			label: 'Deploy to staging/prod',
			onclick: () => deploymentDrawer.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Server
		})

		if (can_write) {
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
	<DetailPageLayout
		isOperator={$userStore?.operator}
		flow_json={{
			value: flow.value,
			summary: flow.summary,
			description: flow.description,
			schema: flow.schema
		}}
		hasStepDetails={Boolean(stepDetail)}
	>
		<svelte:fragment slot="header">
			<DetailPageHeader
				{mainButtons}
				menuItems={getMenuItems(flow)}
				title={defaultIfEmptyString(flow.summary, flow.path)}
				bind:errorHandlerMuted={flow.ws_error_handler_muted}
				scriptOrFlowPath={flow.path}
				errorHandlerKind="flow"
				tag={flow.tag}
			>
				{#if flow?.value?.priority != undefined}
					<div class="hidden md:block">
						<HeaderBadge color="red" variant="outlined" size="xs">
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
			<SplitPanesWrapper>
				<Splitpanes horizontal>
					<Pane size={60} minSize={20}>
						<div class="p-8 w-full max-w-3xl mx-auto gap-2 bg-surface">
							<div class="flex flex-col gap-0.5">
								{#if !emptyString(flow.summary)}
									<span class="text-lg font-semibold">{flow.path}</span>
								{/if}
								<span class="text-sm text-tertiary">
									Edited <TimeAgo date={flow.edited_at ?? ''} /> by {flow.edited_by}
								</span>

								{#if deploymentInProgress}
									<Badge color="yellow">
										<Loader2 size={12} class="inline animate-spin mr-1" />
										Deployment in progress
									</Badge>
								{/if}
								{#if flow.archived}
									<div class="" />
									<Alert type="error" title="Archived">This flow was archived</Alert>
								{/if}

								{#if !emptyString(flow.description)}
									<div class=" break-words whitespace-pre-wrap text-sm">
										<Urlize text={defaultIfEmptyString(flow.description, 'No description')} />
									</div>
								{/if}
							</div>

							<RunForm
								viewKeybinding
								{loading}
								autofocus
								detailed={false}
								bind:isValid
								runnable={flow}
								runAction={runFlow}
								bind:args
								isFlow
								bind:this={runForm}
							/>
						</div>
					</Pane>
					<Pane size={40} minSize={20}>
						<div class="!bg-surface-secondary h-full">
							<FlowGraphViewer
								download
								{flow}
								overflowAuto
								noSide={true}
								on:select={(e) => {
									if (e.detail.id) {
										stepDetail = e.detail
									} else {
										stepDetail = undefined
									}
								}}
							/>
						</div>
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
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
		<svelte:fragment slot="details">
			<div class="p-1">
				<SchemaViewer schema={flow.schema} />
			</div>
		</svelte:fragment>
		<svelte:fragment slot="webhooks">
			<WebhooksPanel
				scopes={[`run:flow/${flow?.path}`]}
				webhooks={{
					async: {
						path: urlAsync
					},
					sync: {
						path: urlSync,
						get_path: urlSync
					}
				}}
				isFlow={true}
				{args}
			/>
		</svelte:fragment>
		<svelte:fragment slot="schedule">
			<RunPageSchedules isFlow={true} path={flow.path ?? ''} {can_write} />
		</svelte:fragment>
		<svelte:fragment slot="cli">
			<div class="p-2">
				<ClipboardPanel content={cliCommand} />
				<CliHelpBox />
			</div>
		</svelte:fragment>

		<svelte:fragment slot="flow_step">
			{#if stepDetail}
				<FlowGraphViewer download {flow} overflowAuto noSide={false} noGraph={true} {stepDetail} />
			{/if}
		</svelte:fragment>
	</DetailPageLayout>
{/if}
