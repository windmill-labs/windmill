<script lang="ts">
	import {
		FlowService,
		JobService,
		WorkspaceService,
		type Flow,
		type FlowModule,
		type TriggersCount,
		type WorkspaceDeployUISettings
	} from '$lib/gen'
	import { canWrite, defaultIfEmptyString, emptyString, urlParamsToObject } from '$lib/utils'
	import { isDeployable, ALL_DEPLOYABLE } from '$lib/utils_deployable'

	import DetailPageLayout from '$lib/components/details/DetailPageLayout.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { Badge as HeaderBadge, Alert } from '$lib/components/common'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import DeployWorkspaceDrawer from '$lib/components/DeployWorkspaceDrawer.svelte'
	import SavedInputsV2 from '$lib/components/SavedInputsV2.svelte'
	import AIFormAssistant from '$lib/components/copilot/AIFormAssistant.svelte'
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
		Pen,
		Eye,
		HistoryIcon,
		LayoutDashboard
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
	import { onDestroy, tick, untrack } from 'svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import TriggersEditor from '$lib/components/triggers/TriggersEditor.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import { setContext } from 'svelte'
	import TriggersBadge from '$lib/components/graph/renderers/triggers/TriggersBadge.svelte'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'
	import FlowAssetsHandler, {
		initFlowGraphAssetsCtx
	} from '$lib/components/flows/FlowAssetsHandler.svelte'
	import { page } from '$app/state'
	import FlowChat from '$lib/components/flows/conversations/FlowChat.svelte'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import NoDirectDeployAlert from '$lib/components/NoDirectDeployAlert.svelte'

	let flow: Flow | undefined = $state()
	let can_write = $state(false)
	let shareModal: ShareModal | undefined = $state()

	let scheduledForStr: string | undefined = $state(undefined)
	let invisible_to_owner: boolean | undefined = $state(undefined)
	let overrideTag: string | undefined = $state(undefined)
	let inputSelected: 'saved' | 'history' | undefined = $state(undefined)
	let jsonView = $state(false)
	let deploymentInProgress = $state(false)
	let deploymentJobId: string | undefined = $state(undefined)

	let intervalId: number | undefined = undefined

	const triggersCount = writable<TriggersCount | undefined>(undefined)

	// Add triggers context store
	const triggersState = $state(
		new Triggers([
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'default_email', path: '', isDraft: false },
			{ type: 'cli', path: '', isDraft: false }
		])
	)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll: writable(false),
		showCaptureHint: writable(undefined),
		triggersState
	})

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => flow?.value.modules ?? [] })
	)

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
				deploymentJobId = undefined
				flow.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			} else if (status.job_id) {
				deploymentJobId = status.job_id
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

	async function runFlowForChat(
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	): Promise<string> {
		const run = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path,
			memoryId: conversationId,
			requestBody: { user_message: userMessage, ...(additionalInputs ?? {}) },
			skipPreprocessor: true
		})
		return run
	}

	let args: Record<string, any> | undefined = $state(undefined)

	let hash = window.location.hash
	if (hash.length > 1) {
		try {
			let searchParams = new URLSearchParams(hash.slice(1))
			let params = [...Object.entries(urlParamsToObject(searchParams))].map(([k, v]) => [
				k,
				JSON.parse(v)
			])
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
					variant: 'subtle',
					unifiedSize: 'md',
					disabled: !showEditButtons,
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
				unifiedSize: 'md',
				variant: 'subtle',
				startIcon: Play
			}
		})

		buttons.push({
			label: `History`,
			buttonProps: {
				onClick: () => flowHistory?.open(),
				unifiedSize: 'md',
				variant: 'subtle',
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
					unifiedSize: 'md',
					variant: 'subtle',
					disabled: !showEditButtons,
					startIcon: LayoutDashboard
				}
			})

			buttons.push({
				label: 'Edit',
				buttonProps: {
					href: `${base}/flows/edit/${path}?nodraft=true`,
					variant: 'accent',
					unifiedSize: 'md',
					disabled: !can_write || !showEditButtons,
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


		if (showEditButtons) {
		menuItems.push({
			label: 'Move/Rename',
			onclick: () => moveDrawer?.openDrawer(flow?.path ?? '', flow?.summary, 'flow'),
			Icon: FolderOpen
		})
		}

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

		if (can_write && showEditButtons) {
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
	let path = $derived(page.params.path ?? '')

	let topSectionHeight = $state(0)
	let paneHeight = $state(0)
	let flowGraphHeight = $state(0)

	let graphMinHeight = $derived.by(() => {
		if (!topSectionHeight || !paneHeight) return 400
		const availableHeight = paneHeight - topSectionHeight - 1 // Account for the separator between the top section and the graph
		return Math.max(400, availableHeight)
	})

	$effect(() => {
		const cliTrigger = triggersState.triggers.find((t) => t.type === 'cli')
		if (cliTrigger) {
			cliTrigger.extra = {
				cliCommand: `wmill flow run ${flow?.path} -d '${JSON.stringify(args)}'`
			}
		}
	})
	$effect(() => {
		if ($workspaceStore && $userStore && page.params.path) {
			if (previousPath !== path) {
				previousPath = path
				untrack(() => {
					loadFlow()
					loadTriggersCount()
					loadTriggers()
				})
			}
		}
	})
	let showEditButtons = $state(false)
	let mainButtons = $derived(getMainButtons(flow, args))
	let chatInputEnabled = $derived(flow?.value?.chat_input_enabled ?? false)
	let shouldUseStreaming = $derived.by(() => {
		const modules = flow?.value?.modules
		const lastModule = modules && modules.length > 0 ? modules[modules.length - 1] : undefined
		return (
			lastModule?.value?.type === 'aiagent' &&
			lastModule?.value?.input_transforms?.streaming?.type === 'static' &&
			lastModule?.value?.input_transforms?.streaming?.value === true
		)
	})
</script>

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
	<FlowHistory bind:this={flowHistory} path={flow.path} onHistoryRestore={loadFlow} />
{/if}

<DetailPageLayout
	bind:selected={rightPaneSelected}
	isOperator={$userStore?.operator}
	forceSmallScreen={chatInputEnabled}
	isChatMode={chatInputEnabled}
	flow_json={{
		value: flow?.value,
		summary: flow?.summary,
		description: flow?.description,
		schema: flow?.schema
	}}
>
	{#snippet header()}
		<DetailPageHeader
			on:seeTriggers={() => {
				rightPaneSelected = 'triggers'
			}}
			{mainButtons}
			menuItems={getMenuItems(flow, deployUiSettings)}
			bind:errorHandlerMuted={
				() => flow?.ws_error_handler_muted ?? false,
				(v) => {
					if (flow !== undefined) flow.ws_error_handler_muted = v
				}
			}
			scriptOrFlowPath={flow?.path ?? ''}
			errorHandlerKind="flow"
			tag={flow?.tag ?? ''}
			summary={flow?.summary}
			path={flow?.path}
			onSaved={can_write
				? async (newPath) => {
						if (newPath !== flow?.path) {
							await goto(`/flows/get/${newPath}?workspace=${$workspaceStore}`)
						} else {
							loadFlow()
						}
					}
				: undefined}
		>
			<!-- @migration-task: migrate this slot by hand, `trigger-badges` is an invalid identifier -->
			{#snippet trigger_badges()}
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
			{/snippet}
			{#if $workspaceStore && flow}
				<Star kind="flow" path={flow.path} summary={flow.summary} />
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
	{/snippet}
	{#snippet form()}
		<div class="px-3">
			<NoDirectDeployAlert onUpdateCanEditStatus={(v) => (showEditButtons = v)} />
		</div>
		{#if flow}
			<div class="flex flex-col h-full bg-surface divide-y" bind:clientHeight={paneHeight}>
				<div bind:clientHeight={topSectionHeight} class={twMerge(chatInputEnabled ? 'h-full' : '')}>
					<div
						class={twMerge(
							'w-full flex flex-col',
							chatInputEnabled ? 'p-3 h-full' : 'max-w-3xl p-6 min-h-[300px] justify-center',
							'mx-auto'
						)}
					>
						{#if flow?.archived}
							<Alert type="error" title="Archived">This flow was archived</Alert>
							<div class="h-4"></div>
						{/if}

						{#if !emptyString(flow?.description)}
							<div class="p-4 rounded-md bg-surface-secondary">
								<GfmMarkdown
									md={defaultIfEmptyString(flow?.description, 'No description')}
									noPadding
								/>
							</div>
							<div class="h-4"></div>
						{/if}

						{#if deploymentInProgress}
							<div class="pb-4" transition:slide={{ duration: 150 }}>
								<HeaderBadge color="yellow">
									<Loader2 size={12} class="inline animate-spin mr-1" />
									Deployment in progress
									{#if deploymentJobId}
										<a
											href="/run/{deploymentJobId}?workspace={$workspaceStore}"
											class="underline"
											target="_blank">view job</a
										>
									{/if}
								</HeaderBadge>
							</div>
						{/if}
						{#if flow.lock_error_logs && flow.lock_error_logs != ''}
							<Alert type="error" title="Deployment failed">
								<p>
									This flow has not been deployed successfully because of the following errors:
								</p>
								<LogViewer content={flow.lock_error_logs} isLoading={false} tag={undefined} />
							</Alert>
							<div class="h-4"></div>
						{/if}

						{#if chatInputEnabled}
							<!-- Chat Layout with Sidebar -->
							<FlowChat
								onRunFlow={runFlowForChat}
								{deploymentInProgress}
								path={flow?.path ?? ''}
								useStreaming={shouldUseStreaming}
								inputSchema={flow?.schema}
							/>
						{:else}
							{@const hasSchema =
								flow.schema && Object.keys(flow.schema.properties ?? {}).length > 0}
							<!-- Normal Mode: Form Layout -->
							<div class="flex flex-col align-left">
								{#if hasSchema || inputSelected}
									<div
										class="flex flex-row justify-between min-h-12"
										transition:slide={{ duration: 150 }}
									>
										<InputSelectedBadge
											onReject={() => {
												savedInputsV2?.resetSelected()
											}}
											{inputSelected}
										/>

										{#if hasSchema}
											<Toggle
												bind:checked={jsonView}
												size="xs"
												options={{
													right: 'JSON',
													rightTooltip: 'Fill args from JSON'
												}}
												lightMode
												on:change={(e) => {
													runForm?.setCode(JSON.stringify(args ?? {}, null, '\t'))
												}}
											/>
										{/if}
									</div>
								{/if}

								{#if flow.schema?.prompt_for_ai !== undefined}
									<AIFormAssistant
										instructions={flow.schema?.prompt_for_ai as string}
										onEditInstructions={() => {
											goto(`/flows/edit/${flow?.path}`)
										}}
										runnableType="flow"
									/>
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
									{jsonView}
								/>
							</div>

							<div class="pt-4 flex flex-col gap-1 w-full items-end">
								<span class="text-2xs text-secondary">
									Edited <TimeAgo date={flow.edited_at ?? ''} noSeconds /> by {flow.edited_by}
								</span>
							</div>
						{/if}
					</div>
				</div>
				{#if !chatInputEnabled}
					<div class="grow min-h-0">
						<FlowGraphViewer
							triggerNode={true}
							download
							{flow}
							noSide={true}
							minHeight={graphMinHeight}
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
							noBorder={true}
						/>
					</div>
				{/if}
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
			args={args ?? {}}
			bind:inputSelected
			on:selected_args={(e) => {
				const nargs = JSON.parse(JSON.stringify(e.detail))
				args = nargs
				if (jsonView) {
					runForm?.setCode(JSON.stringify(args ?? {}, null, '\t'))
				}
			}}
		/>
	{/snippet}

	{#snippet flow_step()}
		{#if flow}
			{#if stepDetail}
				<FlowGraphViewerStep schema={flow.schema} {stepDetail} />
			{/if}
		{/if}
	{/snippet}
	{#snippet triggers()}
		{#if flow}
			<TriggersEditor
				{args}
				runnableVersion={flow.version_id?.toString()}
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

	{#snippet flow_graph()}
		{#if flow}
			<div class="h-full overflow-auto" bind:clientHeight={flowGraphHeight}>
				<FlowGraphViewer
					triggerNode={true}
					download
					{flow}
					noSide={false}
					noBorder
					minHeight={flowGraphHeight}
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
		{/if}
	{/snippet}
</DetailPageLayout>

<FlowAssetsHandler
	modules={flow?.value.modules ?? []}
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
