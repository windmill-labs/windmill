<script lang="ts">
	import { run as run_1 } from 'svelte/legacy'

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
	import { canWrite, defaultIfEmptyString, emptyString } from '$lib/utils'
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
	import SavedInputsV2 from '$lib/components/SavedInputsV2.svelte'
	import {
		FolderOpen,
		Archive,
		Trash,
		ChevronUpSquare,
		Share,
		Loader2,
		GitFork,
		Play,
		History,
		Columns,
		Pen,
		Eye,
		HistoryIcon
	} from 'lucide-svelte'

	import DetailPageHeader from '$lib/components/details/DetailPageHeader.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import { createAppFromFlow } from '$lib/components/details/createAppFromScript'
	import { importStore } from '$lib/components/apps/store'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import FlowGraphViewerStep from '$lib/components/FlowGraphViewerStep.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import FlowHistory from '$lib/components/flows/FlowHistory.svelte'
	import Star from '$lib/components/Star.svelte'

	import { writable } from 'svelte/store'
	import InputSelectedBadge from '$lib/components/schema/InputSelectedBadge.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { onDestroy, tick } from 'svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import TriggersEditor from '$lib/components/triggers/TriggersEditor.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { setContext } from 'svelte'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'

	let flow: Flow | undefined = $state()
	let can_write = false
	let shareModal: ShareModal | undefined = $state()

	let scheduledForStr: string | undefined = $state(undefined)
	let invisible_to_owner: boolean | undefined = $state(undefined)
	let overrideTag: string | undefined = $state(undefined)
	let inputSelected: 'saved' | 'history' | undefined = $state(undefined)
	let jsonView = $state(false)
	let deploymentInProgress = $state(false)

	let intervalId: NodeJS.Timeout | undefined = undefined

	const triggersCount = writable<TriggersCount | undefined>(undefined)

	// Add triggers context store
	const triggersState = $state(
		new Triggers([
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'email', path: '', isDraft: false },
			{ type: 'cli', path: '', isDraft: false }
		])
	)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState
	})

	let previousPath: string | undefined = $state(undefined)

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

	let starred: boolean | undefined = $state(undefined)

	async function loadTriggersCount() {
		$triggersCount = await FlowService.getTriggersCountOfFlow({
			workspace: $workspaceStore!,
			path
		})
	}

	async function loadTriggers(): Promise<void> {
		await triggersState.fetchTriggers(
			triggersCount,
			$workspaceStore,
			path,
			true,
			undefined,
			$userStore
		)
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
		intervalId && clearInterval(intervalId)
		deploymentInProgress = flow.lock_error_logs == ''
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
		can_write = canWrite(flow.path, flow.extra_perms!, $userStore)
	}

	let isValid = $state(true)
	let loading = $state(false)

	async function syncer(): Promise<void> {
		if (flow) {
			const status = await FlowService.getFlowDeploymentStatus({
				workspace: $workspaceStore!,
				path: flow.path
			})
			if (status.lock_error_logs == undefined || status.lock_error_logs != '') {
				deploymentInProgress = false
				flow.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			}
		}
	}

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

	let args: Record<string, any> = $state.raw({})

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

	let moveDrawer: MoveDrawer | undefined = $state()
	let deploymentDrawer: DeployWorkspaceDrawer | undefined = $state()
	let runForm: RunForm | undefined = $state()

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
			label: `Runs`,
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
				label: 'Build app',
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

	let deployUiSettings: WorkspaceDeployUISettings | undefined = $state(undefined)

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
			onclick: () => shareModal?.openDrawer(flow?.path ?? '', 'flow'),
			Icon: Share,
			disabled: !can_write
		})

		menuItems.push({
			label: 'Move/Rename',
			onclick: () => moveDrawer?.openDrawer(flow?.path ?? '', flow?.summary, 'flow'),
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
				onclick: () => deploymentDrawer?.openDrawer(flow?.path ?? '', 'flow'),
				Icon: ChevronUpSquare
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

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

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
	let stepDetail: FlowModule | string | undefined = $state(undefined)

	let rightPaneSelected = $state('saved_inputs')
	let savedInputsV2: SavedInputsV2 | undefined = $state(undefined)
	let flowHistory: FlowHistory | undefined = $state(undefined)
	let path = $derived($page.params.path)
	run_1(() => {
		const cliTrigger = triggersState.triggers.find((t) => t.type === 'cli')
		if (cliTrigger) {
			cliTrigger.extra = {
				cliCommand: `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`
			}
		}
	})
	run_1(() => {
		if ($workspaceStore && $userStore && $page.params.path) {
			if (previousPath !== path) {
				previousPath = path
				loadFlow()
				loadTriggersCount()
				loadTriggers()
			}
		}
	})
	let mainButtons = $derived(getMainButtons(flow, args))
</script>

<Skeleton
	class="!max-w-7xl !px-4 sm:!px-6 md:!px-8"
	loading={!flow}
	layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }], 0.2, [{ h: 1, w: 30 }]]}
/>
<svelte:window onkeydown={onKeyDown} />
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
		bind:selected={rightPaneSelected}
		isOperator={$userStore?.operator}
		flow_json={{
			value: flow.value,
			summary: flow.summary,
			description: flow.description,
			schema: flow.schema
		}}
	>
		{#snippet header()}
			{#if flow}
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
					<!-- @migration-task: migrate this slot by hand, `trigger-badges` is an invalid identifier -->
					<svelte:fragment slot="trigger-badges">
						<TriggersBadge
							showOnlyWithCount={true}
							showDraft={false}
							{path}
							newItem={false}
							isFlow
							selected={rightPaneSelected == 'triggers'}
							onSelect={async (triggerIndex: number) => {
								rightPaneSelected = 'triggers'
								await tick()
								triggersState.selectedTriggerIndex = triggerIndex
							}}
							small={false}
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
			{/if}
		{/snippet}
		{#snippet form()}
			{#if flow}
				<div class="flex-col flex h-full justify-between">
					<div class="p-8 w-full max-w-3xl mx-auto gap-2 bg-surface">
						<div class="mb-1">
							{#if !emptyString(flow?.description)}
								<GfmMarkdown md={defaultIfEmptyString(flow?.description, 'No description')} />
							{/if}
						</div>

						{#if deploymentInProgress}
							<HeaderBadge color="yellow">
								<Loader2 size={12} class="inline animate-spin mr-1" />
								Deployment in progress
							</HeaderBadge>
						{/if}
						{#if flow.lock_error_logs && flow.lock_error_logs != ''}
							<div class="bg-red-100 dark:bg-red-700 border-l-4 border-red-500 p-4" role="alert">
								<p class="font-bold">Error deploying this flow</p>
								<p>
									This flow has not been deployed successfully because of the following errors:
								</p>
								<LogViewer content={flow.lock_error_logs} isLoading={false} tag={undefined} />
							</div>
						{/if}

						<div class="flex flex-col align-left">
							<div class="flex flex-row justify-between">
								<InputSelectedBadge
									on:click={() => {
										savedInputsV2?.resetSelected()
									}}
									{inputSelected}
								/>
								<Toggle
									bind:checked={jsonView}
									label="JSON View"
									size="xs"
									options={{
										right: 'JSON',
										rightTooltip: 'Fill args from JSON'
									}}
									lightMode
									on:change={(e) => {
										runForm?.setCode(JSON.stringify(args, null, '\t'))
									}}
								/>
							</div>

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
								{jsonView}
							/>
						</div>

						<div class="py-10"></div>

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
								<div class=""></div>
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
			{/if}
		{/snippet}
		{#snippet save_inputs()}
			<SavedInputsV2
				bind:this={savedInputsV2}
				schema={flow?.schema}
				{jsonView}
				flowPath={flow?.path}
				{isValid}
				{args}
				bind:inputSelected
				on:selected_args={(e) => {
					const nargs = JSON.parse(JSON.stringify(e.detail))
					args = nargs
				}}
			/>
		{/snippet}
		{#snippet flow_step()}
			{#if flow && stepDetail}
				<FlowGraphViewerStep schema={flow.schema} {stepDetail} />
			{/if}
		{/snippet}
		{#snippet triggers()}
			{#if flow}
				<TriggersEditor
					initialPath={flow.path}
					currentPath={flow.path}
					noEditor={true}
					newItem={false}
					isFlow={true}
					schema={flow.schema}
					isDeployed={true}
					noCapture={true}
					isEditor={false}
				/>
			{/if}
		{/snippet}
	</DetailPageLayout>
{/if}
