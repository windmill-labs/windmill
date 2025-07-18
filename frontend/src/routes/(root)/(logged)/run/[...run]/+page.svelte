<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { page } from '$app/stores'
	import { base } from '$lib/base'
	import {
		JobService,
		type Job,
		ScriptService,
		type Script,
		type WorkflowStatus,
		type NewScript,
		ConcurrencyGroupsService,
		MetricsService,
		type ScriptArgs
	} from '$lib/gen'
	import {
		canWrite,
		computeSharableHash,
		copyToClipboard,
		displayDate,
		emptyString,
		encodeState,
		isFlowPreview,
		isNotFlow,
		isScriptPreview,
		truncateHash,
		truncateRev
	} from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import {
		Activity,
		ArrowRight,
		Calendar,
		CheckCircle2,
		Circle,
		FastForward,
		Hourglass,
		List,
		Pen,
		RefreshCw,
		TimerOff,
		Trash,
		XCircle,
		Code2,
		ClipboardCopy,
		MoreVertical,
		GitBranch
	} from 'lucide-svelte'

	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import {
		enterpriseLicense,
		initialArgsStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { ActionRow, Button, Skeleton, Tab, Alert, DrawerContent } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import { forLater } from '$lib/forLater'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import MemoryFootprintViewer from '$lib/components/MemoryFootprintViewer.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkflowTimeline from '$lib/components/WorkflowTimeline.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import PreprocessedArgsDisplay from '$lib/components/runs/PreprocessedArgsDisplay.svelte'
	import ExecutionDuration from '$lib/components/ExecutionDuration.svelte'
	import CustomPopover from '$lib/components/CustomPopover.svelte'
	import { isWindmillTooBigObject } from '$lib/components/job_args'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import { setContext, untrack } from 'svelte'
	import WorkerHostname from '$lib/components/WorkerHostname.svelte'
	import FlowAssetsHandler, {
		initFlowGraphAssetsCtx
	} from '$lib/components/flows/FlowAssetsHandler.svelte'
	import JobAssetsViewer from '$lib/components/assets/JobAssetsViewer.svelte'

	let job: Job | undefined = $state()
	let jobUpdateLastFetch: Date | undefined = $state()

	let scriptProgress: number | undefined = $state(undefined)
	let currentJobIsLongRunning: boolean = $state(false)

	let viewTab: 'result' | 'logs' | 'code' | 'stats' | 'assets' = $state('result')
	let selectedJobStep: string | undefined = $state(undefined)
	let branchOrIterationN: number = $state(0)

	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let restartBranchNames: [number, string][] = []

	let testIsLoading = $state(false)
	let testJobLoader: TestJobLoader | undefined = $state(undefined)

	let persistentScriptDrawer: PersistentScriptDrawer | undefined = $state(undefined)

	let showExplicitProgressTip: boolean = $state(
		(localStorage.getItem('hideExplicitProgressTip') ?? 'false') == 'false'
	)

	let lastJobId: string | undefined = $state(undefined)
	let concurrencyKey: string | undefined = $state(undefined)

	setContext(
		'FlowGraphAssetContext',
		initFlowGraphAssetsCtx({ getModules: () => job?.raw_flow?.modules ?? [] })
	)

	async function getConcurrencyKey(job: Job | undefined) {
		if (!job) return
		lastJobId = job.id
		concurrencyKey = await ConcurrencyGroupsService.getConcurrencyKey({ id: job.id })
	}

	async function deleteCompletedJob(id: string): Promise<void> {
		await JobService.deleteCompletedJob({ workspace: $workspaceStore!, id })
		getJob()
	}

	async function cancelJob(id: string) {
		try {
			if (forceCancel) {
				await JobService.forceCancelQueuedJob({ workspace: $workspaceStore!, id, requestBody: {} })
				setTimeout(getJob, 5000)
			} else {
				await JobService.cancelQueuedJob({ workspace: $workspaceStore!, id, requestBody: {} })
			}
			sendUserToast(`job ${id} canceled`)
		} catch (err) {
			sendUserToast('could not cancel job', true)
		}
	}

	async function restartFlow(
		id: string | undefined,
		stepId: string | undefined,
		branchOrIterationN: number
	) {
		if (id === undefined || stepId === undefined) {
			return
		}
		let run = await JobService.restartFlowAtStep({
			workspace: $workspaceStore!,
			id,
			stepId,
			branchOrIterationN,
			requestBody: {}
		})
		await goto('/run/' + run + '?workspace=' + $workspaceStore)
	}

	// If we get results, focus on that tab. Else, focus on logs
	function initView(): void {
		if (job && 'result' in job && job.result != undefined) {
			viewTab = 'result'
		} else if (viewTab == 'result') {
			viewTab = 'logs'
		}
	}

	async function getJob() {
		await testJobLoader?.watchJob($page.params.run)
		initView()
	}

	function onSelectedJobStepChange() {
		if (selectedJobStep !== undefined && job?.flow_status?.modules !== undefined) {
			selectedJobStepIsTopLevel =
				job?.flow_status?.modules.map((m) => m.id).indexOf(selectedJobStep) >= 0
			let moduleDefinition = job?.raw_flow?.modules.find((m) => m.id == selectedJobStep)
			if (moduleDefinition?.value.type == 'forloopflow') {
				selectedJobStepType = 'forloop'
			} else if (moduleDefinition?.value.type == 'branchall') {
				selectedJobStepType = 'branchall'
				moduleDefinition?.value.branches.forEach((branch, idx) => {
					restartBranchNames.push([
						idx,
						emptyString(branch.summary) ? `Branch #${idx}` : branch.summary!
					])
				})
			} else {
				selectedJobStepType = 'single'
			}
		}
	}

	let persistentScriptDefinition: Script | undefined = $state(undefined)

	async function onJobLoaded() {
		// We want to set up scriptProgress once job is loaded
		// We need this to show progress bar if job has progress and is finished
		if (job && job.type == 'CompletedJob') {
			// If error occured and job is completed
			// than we fetch progress from server to display on what progress did it fail
			// Could be displayed after run or as a historical page
			// If opening page without running job (e.g. reloading page after run) progress will be displayed instantly
			MetricsService.getJobProgress({
				workspace: job.workspace_id ?? 'NO_WORKSPACE',
				id: job.id
			}).then((progress) => {
				// Returned progress is not always 100%, could be 65%, 33%, anything
				// Its ok if its a failure and we want to keep that value
				// But we want progress to be 100% if job has been succeeded
				scriptProgress = progress
			})
		}

		if (
			job &&
			job.job_kind === 'script' &&
			job.script_hash &&
			persistentScriptDefinition === undefined
		) {
			const script = await ScriptService.getScriptByHash({
				workspace: $workspaceStore!,
				hash: job.script_hash
			})
			if (script.restart_unless_cancelled ?? false) {
				persistentScriptDefinition = script
			}
		}
	}

	function onRunsPageChangeWithLoader() {
		forceCancel = false
		getJob()
	}

	function onRunsPageChange() {
		job = undefined
		persistentScriptDefinition = undefined
	}

	let notfound = $state(false)
	let forceCancel = $state(false)

	let debugViewer: Drawer | undefined = $state(undefined)
	let debugContent: any = $state(undefined)
	async function debugInfo() {
		if (job?.id) {
			debugContent = await JobService.getFlowDebugInfo({ workspace: $workspaceStore!, id: job?.id })
			debugViewer?.openDrawer()
		} else {
			sendUserToast('Job has no id', true)
		}
	}

	function removeSensitiveInfos(
		jobs: { [job: string]: { args: any; result: any; logs: string } },
		redactSensitive: boolean
	) {
		if (!redactSensitive) {
			return jobs
		}
		if (jobs === undefined || typeof jobs !== 'object') {
			return []
		}
		return Object.fromEntries(
			Object.entries(jobs).map(([k, job]) => {
				return [
					k,
					{
						...job,
						args: '[redacted]',
						result: '[redacted]',
						logs: '[redacted]'
					}
				]
			})
		)
	}

	let redactSensitive = $state(false)

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}

	function forkPreview() {
		if (isFlowPreview(job?.job_kind)) {
			$initialArgsStore = job?.args
			const state = {
				flow: { value: job?.raw_flow },
				path: job?.script_path + '_fork'
			}
			window.open(`/flows/add#${encodeState(state)}`)
		} else {
			$initialArgsStore = job?.args
			let n: NewScript = {
				path: job?.script_path + '_fork',
				summary: 'Fork of preview of ' + job?.script_path,
				language: job?.language as NewScript['language'],
				description: '',
				content: job?.raw_code ?? ''
			}
			window.open(`/scripts/add#${encodeState(n)}`)
		}
	}

	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	let runImmediatelyLoading = $state(false)
	async function runImmediately() {
		runImmediatelyLoading = true
		try {
			let args = job?.args as ScriptArgs
			if (isWindmillTooBigObject(args)) {
				args = (await JobService.getJobArgs({
					workspace: $workspaceStore!,
					id: job?.id!
				})) as ScriptArgs
			}

			const commonArgs = {
				workspace: $workspaceStore!,
				requestBody: args
			}
			if (job?.job_kind == 'script' || job?.job_kind == 'flow') {
				let id

				if (job?.job_kind == 'script') {
					id = await JobService.runScriptByHash({
						...commonArgs,
						hash: job.script_hash!
					})
				} else {
					id = await JobService.runFlowByPath({
						...commonArgs,
						path: job.script_path!
					})
				}

				await goto('/run/' + id + '?workspace=' + $workspaceStore)
			} else {
				sendUserToast('Cannot run this job immediately', true)
			}
		} finally {
			runImmediatelyLoading = false
		}
	}
	$effect(() => {
		job?.logs == undefined &&
			job &&
			viewTab == 'logs' &&
			isNotFlow(job?.job_kind) &&
			testJobLoader?.getLogs()
	})
	$effect(() => {
		job?.id && lastJobId !== job.id && untrack(() => getConcurrencyKey(job))
	})
	$effect(() => {
		$workspaceStore && $page.params.run && untrack(() => onRunsPageChange())
	})
	$effect(() => {
		$workspaceStore &&
			$page.params.run &&
			testJobLoader &&
			untrack(() => onRunsPageChangeWithLoader())
	})
	$effect(() => {
		selectedJobStep !== undefined && untrack(() => onSelectedJobStepChange())
	})
	$effect(() => {
		job && untrack(() => onJobLoaded())
	})
</script>

<HighlightTheme />

<ScheduleEditor bind:this={scheduleEditor} />

{#if (job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)) && job?.['running'] && job?.parent_job == undefined}
	<Drawer bind:this={debugViewer} size="800px">
		<DrawerContent title="Debug Detail" on:close={debugViewer.closeDrawer}>
			{#snippet actions()}
				<div class="flex items-center gap-1">
					<div class="w-60 pt-2">
						<Toggle bind:checked={redactSensitive} options={{ right: 'Redact args/result/logs' }} />
					</div>
					<Button
						on:click={() =>
							copyToClipboard(
								JSON.stringify(removeSensitiveInfos(debugContent, redactSensitive), null, 4)
							)}
						color="light"
						size="xs"
					>
						<div class="flex gap-2 items-center">Copy <ClipboardCopy /> </div>
					</Button>
				</div>
			{/snippet}
			<pre
				><code class="text-2xs p-2">
					<Highlight
						language={json}
						code={JSON.stringify(removeSensitiveInfos(debugContent, redactSensitive), null, 4)}
					/>
			</code></pre
			>
		</DrawerContent>
	</Drawer>
{/if}
{#if !job || (job?.job_kind != 'flow' && job?.job_kind != 'flownode' && job?.job_kind != 'flowpreview')}
	<TestJobLoader
		lazyLogs
		bind:scriptProgress
		on:done={() => job?.['result'] != undefined && (viewTab = 'result')}
		bind:this={testJobLoader}
		bind:isLoading={testIsLoading}
		bind:job
		bind:jobUpdateLastFetch
		workspaceOverride={$workspaceStore}
		bind:notfound
	/>
{/if}

<Portal name="persistent-run">
	<PersistentScriptDrawer bind:this={persistentScriptDrawer} />
</Portal>

{#if notfound || (job?.workspace_id != undefined && $workspaceStore != undefined && job?.workspace_id != $workspaceStore)}
	<div class="max-w-7xl px-4 mx-auto w-full">
		<div class="flex flex-col gap-6">
			<h1 class="text-red-400 mt-6">Job {$page.params.run} not found in {$workspaceStore}</h1>
			<h2>Are you in the right workspace?</h2>
			<div class="flex flex-col gap-2">
				{#each $userWorkspaces as workspace}
					<div>
						<Button
							variant="border"
							on:click={() => {
								goto(`/run/${$page.params.run}?workspace=${workspace.id}`)
							}}
						>
							See in {workspace.name}
						</Button>
					</div>
				{/each}
				<div>
					<Button href="{base}/runs">Go to runs page</Button>
				</div>
			</div>
		</div>
	</div>
{:else}
	<Skeleton
		class="max-w-7xl px-4 mx-auto w-full"
		loading={!job}
		layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }]]}
	/>
	<ActionRow class="max-w-7xl px-4 mx-auto w-full">
		{#snippet left()}
			{@const isScript = job?.job_kind === 'script'}
			{@const runsHref = `/runs/${job?.script_path}${!isScript ? '?jobKind=flow' : ''}`}
			<div class="flex gap-2 items-center">
				{#if job && 'deleted' in job && !job?.deleted && ($superadmin || ($userStore?.is_admin ?? false))}
					<Dropdown
						items={[
							{
								displayName: 'Delete result, logs and args (admin only)',
								action: () => {
									job?.id && deleteCompletedJob(job.id)
								},
								type: 'delete'
							}
						]}
					>
						{#snippet buttonReplacement()}
							<Button nonCaptureEvent variant="border" size="sm" startIcon={{ icon: Trash }} />
						{/snippet}
					</Dropdown>
					{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
						<Button
							href={runsHref}
							variant="border"
							color="blue"
							size="sm"
							startIcon={{ icon: List }}
						>
							View runs
						</Button>
					{/if}
				{/if}
			</div>
		{/snippet}
		{#snippet right()}
			{@const stem = `/${job?.job_kind}s`}
			{@const isScript = job?.job_kind === 'script'}
			{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			{#if (job?.job_kind == 'flow' || isFlowPreview(job?.job_kind)) && job?.['running'] && job?.parent_job == undefined}
				<div class="inline">
					<Dropdown
						items={[
							{
								displayName: 'Show Flow Debug Info',
								action: () => {
									debugInfo()
								}
							}
						]}
						class="h-auto"
					>
						{#snippet buttonReplacement()}
							<Button nonCaptureEvent size="xs" color="light">
								<div class="flex flex-row items-center">
									<MoreVertical size={14} />
								</div>
							</Button>
						{/snippet}
					</Dropdown>
				</div>
			{/if}
			{#if isFlowPreview(job?.job_kind) || isScriptPreview(job?.job_kind)}
				<Button
					color="dark"
					size="md"
					variant="border"
					startIcon={{ icon: GitBranch }}
					on:click={forkPreview}
				>
					Fork {isFlowPreview(job?.job_kind) ? 'flow' : 'code'} preview
				</Button>
			{/if}
			{#if persistentScriptDefinition !== undefined}
				<Button
					color="blue"
					size="md"
					startIcon={{ icon: Activity }}
					on:click={() => {
						persistentScriptDrawer?.open?.(persistentScriptDefinition)
					}}
				>
					Current runs
				</Button>
			{/if}
			{#if job?.type != 'CompletedJob' && (!job?.schedule_path || job?.['running'] == true)}
				{#if !forceCancel}
					<Button
						color="red"
						size="md"
						startIcon={{ icon: TimerOff }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
								setTimeout(() => {
									forceCancel = true
								}, 3001)
							}
						}}
					>
						Cancel
					</Button>
				{:else}
					<Button
						color="red"
						size="md"
						startIcon={{ icon: TimerOff }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
							}
						}}
					>
						Force Cancel
					</Button>
				{/if}
			{/if}
			{#if job?.schedule_path}
				<Button
					size="sm"
					on:click={() => {
						if (!job || !job.schedule_path) {
							return
						}
						scheduleEditor?.openEdit(job.schedule_path, job.job_kind == 'flow')
					}}
					startIcon={{ icon: Calendar }}>Edit schedule</Button
				>
			{/if}
			{#if job?.type === 'CompletedJob' && job?.job_kind === 'flow' && selectedJobStep !== undefined && selectedJobStepIsTopLevel}
				{#if selectedJobStepType == 'single'}
					<Button
						title={`Re-start this flow from step ${selectedJobStep} (included). ${
							!$enterpriseLicense ? ' This is a feature only available in enterprise edition.' : ''
						}`}
						variant="border"
						color="blue"
						disabled={!$enterpriseLicense}
						on:click|once={() => {
							restartFlow(job?.id, selectedJobStep, 0)
						}}
						startIcon={{ icon: RefreshCw }}
					>
						Re-start from
						<Badge baseClass="ml-1" color="indigo">
							{selectedJobStep}
						</Badge>
						{#if !$enterpriseLicense}
							(EE)
						{/if}
					</Button>
				{:else}
					<Popover
						floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
						contentClasses="p-4"
					>
						{#snippet trigger()}
							<Button
								title={`Re-start this flow from step ${selectedJobStep} (included). ${
									!$enterpriseLicense
										? ' This is a feature only available in enterprise edition.'
										: ''
								}`}
								variant="border"
								color="blue"
								disabled={!$enterpriseLicense}
								startIcon={{ icon: RefreshCw }}
								nonCaptureEvent={true}
							>
								Re-start from
								<Badge baseClass="ml-1" color="indigo">
									{selectedJobStep}
								</Badge>
							</Button>
						{/snippet}
						{#snippet content()}
							<label class="block text-primary">
								<div class="pb-1 text-sm text-secondary"
									>{selectedJobStepType == 'forloop' ? 'From iteration #:' : 'From branch:'}</div
								>
								<div class="flex w-full">
									{#if selectedJobStepType === 'forloop'}
										<input
											type="number"
											min="0"
											bind:value={branchOrIterationN}
											class="!w-32 grow"
											onclick={stopPropagation(() => {})}
										/>
									{:else}
										<select
											bind:value={branchOrIterationN}
											class="!w-32 grow"
											onclick={stopPropagation(() => {})}
										>
											{#each restartBranchNames as [branchIdx, branchName]}
												<option value={branchIdx}>{branchName}</option>
											{/each}
										</select>
									{/if}

									<Button
										size="xs"
										color="blue"
										buttonType="button"
										btnClasses="!p-1 !w-[34px] !ml-1"
										aria-label="Restart flow"
										on:click|once={() => {
											restartFlow(job?.id, selectedJobStep, branchOrIterationN)
										}}
									>
										<ArrowRight size={18} />
									</Button>
								</div>
							</label>
						{/snippet}
					</Popover>
				{/if}
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				<CustomPopover noPadding appearTimeout={0}>
					<Button
						on:click|once={() => {
							goto(viewHref + `#${computeSharableHash(job?.args)}`)
						}}
						color="blue"
						size="sm"
						startIcon={{ icon: RefreshCw }}>Run again</Button
					>
					{#snippet overlay()}
						<div class="flex flex-row gap-2">
							<Button size="xs" loading={runImmediatelyLoading} on:click={() => runImmediately()}>
								Run immediately with same args
							</Button>
						</div>
					{/snippet}
				</CustomPopover>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				{#if !$userStore?.operator}
					{#if canWrite(job?.script_path ?? '', {}, $userStore)}
						<Button
							on:click|once={() => {
								$initialArgsStore = job?.args
								goto(`${stem}/edit/${job?.script_path}${isScript ? `` : `?nodraft=true`}`)
							}}
							color="blue"
							size="sm"
							startIcon={{ icon: Pen }}>Edit</Button
						>
					{/if}
				{/if}
				<Button
					href={viewHref}
					color="blue"
					size="sm"
					startIcon={{
						icon:
							job?.job_kind === 'script' ? Code2 : job?.job_kind === 'flow' ? BarsStaggered : Code2
					}}
				>
					View {job?.job_kind}
				</Button>
			{/if}
		{/snippet}
	</ActionRow>
	<div class="w-full">
		<h1
			class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-6 max-w-7xl mx-auto px-4"
		>
			<div class="flex flex-row flex-wrap gap-6 items-center">
				{#if job}
					{#if 'success' in job && job.success}
						{#if job.is_skipped}
							<FastForward class="text-green-600" size={14} />
						{:else}
							<CheckCircle2 class="text-green-600" size={14} />
						{/if}
					{:else if job && 'success' in job}
						<XCircle class="text-red-700" size={14} />
					{:else if job && 'running' in job && job.running && job.suspend}
						<Hourglass class="text-violet-500" size={14} />
					{:else if job && 'running' in job && job.running}
						<Circle class="text-yellow-500 fill-current" size={14} />
					{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
						<Calendar class="text-secondary" size={14} />
					{:else if job && 'running' in job && job.scheduled_for}
						<Hourglass class="text-tertiary" size={14} />
					{/if}
					{job.script_path ?? (job.job_kind == 'dependencies' ? 'lock dependencies' : 'No path')}
					<div class="flex flex-row gap-2 items-center flex-wrap">
						{#if job.script_hash}
							{#if job.job_kind == 'script'}
								<a href="{base}/scripts/get/{job.script_hash}?workspace={$workspaceStore}"
									><Badge color="gray">{truncateHash(job.script_hash)}</Badge></a
								>
							{:else}
								<div>
									<Badge color="gray">{truncateHash(job.script_hash)}</Badge>
								</div>
							{/if}
						{/if}
						{#if job && 'job_kind' in job}
							<div>
								<Badge color="blue">{job.job_kind}</Badge>
							</div>
						{/if}
						{#if job && job.flow_status && job.job_kind === 'script'}
							<PreprocessedArgsDisplay preprocessed={job.preprocessed} />
						{/if}
						{#if persistentScriptDefinition}
							<button onclick={() => persistentScriptDrawer?.open?.(persistentScriptDefinition)}
								><Badge color="red">persistent</Badge></button
							>
						{/if}
						{#if job && 'priority' in job}
							<div>
								<Badge color="blue">priority: {job.priority}</Badge>
							</div>
						{/if}
						{#if job.tag && !['deno', 'python3', 'flow', 'other', 'go', 'postgresql', 'mysql', 'bigquery', 'snowflake', 'mssql', 'graphql', 'oracledb', 'nativets', 'bash', 'powershell', 'php', 'rust', 'other', 'ansible', 'csharp', 'nu', 'java', 'duckdb', 'dependency'].includes(job.tag)}
							<!-- for related places search: ADD_NEW_LANG -->
							<div>
								<Badge color="indigo">Tag: {job.tag}</Badge>
							</div>
						{/if}
						{#if !job.visible_to_owner}
							<div>
								<Badge color="red">
									only visible to you
									<Tooltip>
										{#snippet text()}
											The option to hide this run from the owner of this script or flow was
											activated
										{/snippet}
									</Tooltip>
								</Badge>
							</div>
						{/if}
						{#if job?.['labels'] && Array.isArray(job?.['labels']) && job?.['labels'].length > 0}
							{#each job?.['labels'] as label}
								<div>
									<Badge>Label: {label}</Badge>
								</div>
							{/each}
						{/if}
						{#if concurrencyKey}
							<div>
								<Tooltip notClickable>
									{#snippet text()}
										This job has concurrency limits enabled with the key
										<a
											href={`${base}/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}
										>
											{concurrencyKey}
										</a>
									{/snippet}
									<a
										href={`${base}/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}
									>
										<Badge>Concurrency: {truncateRev(concurrencyKey, 20)}</Badge></a
									>
								</Tooltip>
							</div>
						{/if}
						{#if job?.worker}
							<div>
								<Tooltip notClickable>
									{#snippet text()}
										worker:
										<a href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}>
											{job?.worker}
										</a><br />
										<WorkerHostname worker={job?.worker!} minTs={job?.['created_at']} />
									{/snippet}
									<a href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}>
										<Badge>Worker: {truncateRev(job?.worker, 20)}</Badge></a
									>
								</Tooltip>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</h1>
		{#if job?.['deleted']}
			<div class="max-w-7xl mx-auto w-full px-4">
				<Alert type="error" title="Deleted">
					The content of this run was deleted (by an admin, no less)
				</Alert>
			</div>
			<div class="my-4"></div>
		{/if}

		<!-- Arguments and actions -->
		<div
			class="flex flex-col gap-y-8 sm:grid sm:grid-cols-3 sm:gap-10 max-w-7xl mx-auto w-full px-4"
		>
			<div class="col-span-2">
				<JobArgs
					workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
					id={job?.id}
					args={job?.args}
				/>
			</div>
			<div>
				<Skeleton loading={!job} layout={[[9.5]]} />
				{#if job}
					<FlowMetadata {job} {scheduleEditor} />
					{#if currentJobIsLongRunning && showExplicitProgressTip && !scriptProgress && 'running' in job}
						<Alert
							class="mt-4 p-1 flex flex-row relative text-center"
							size="xs"
							type="info"
							title="tip: Track progress of longer jobs"
							tooltip="For better transparency and verbosity, you can try setting progress from within the script."
							documentationLink="https://www.windmill.dev/docs/advanced/explicit_progress"
						>
							<button
								type="button"
								onclick={() => {
									localStorage.setItem('hideExplicitProgressTip', 'true')
									showExplicitProgressTip = false
								}}
								class="absolute m-2 top-0 right-0 inline-flex rounded-md bg-surface-secondary text-gray-400 hover:text-tertiary focus:outline-none"
							>
								<span class="sr-only">Close</span>
								<svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
									<path
										d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"
									/>
								</svg>
							</button>
						</Alert>
					{/if}
				{/if}
			</div>
		</div>

		{#if job?.['scheduled_for'] && forLater(job?.['scheduled_for'])}
			<div class="max-w-7xl mx-auto w-full px-4">
				<h2 class="mt-10">Scheduled to be executed later: {displayDate(job?.['scheduled_for'])}</h2>
			</div>
		{/if}
		{#if isNotFlow(job?.job_kind)}
			{#if ['python3', 'bun', 'deno'].includes(job?.language ?? '') && (job?.job_kind == 'script' || isScriptPreview(job?.job_kind))}
				<ExecutionDuration bind:job bind:longRunning={currentJobIsLongRunning} />
			{/if}
			<div class="max-w-7xl mx-auto w-full px-4 mb-10">
				{#if job?.flow_status && typeof job.flow_status == 'object' && !('_metadata' in job.flow_status)}
					<div class="mt-10"></div>
					<WorkflowTimeline
						flow_status={asWorkflowStatus(job.flow_status)}
						flowDone={job.type == 'CompletedJob'}
					/>
				{/if}
				{#if scriptProgress}
					<JobProgressBar {job} {scriptProgress} class="py-4" hideStepTitle={true} />
				{/if}
				<!-- Logs and outputs-->
				<div class="mr-2 sm:mr-0 mt-12">
					<Tabs bind:selected={viewTab}>
						<Tab value="result">Result</Tab>
						<Tab value="logs">Logs</Tab>
						<Tab value="stats">Metrics</Tab>
						<Tab value="assets">Assets</Tab>
						{#if isScriptPreview(job?.job_kind)}
							<Tab value="code">Code</Tab>
						{/if}
					</Tabs>

					<Skeleton loading={!job} layout={[[5]]} />
					{#if job}
						<div class="flex flex-row border rounded-md p-2 mt-2 overflow-auto min-h-[600px]">
							{#if viewTab == 'logs'}
								<div class="w-full">
									<LogViewer
										jobId={job.id}
										duration={job?.['duration_ms']}
										mem={job?.['mem_peak']}
										isLoading={job?.['running'] == false}
										content={job?.logs}
										tag={job?.tag}
									/>
								</div>
							{:else if viewTab == 'assets'}
								<div class="w-full">
									<JobAssetsViewer {job} />
								</div>
							{:else if viewTab == 'code'}
								{#if job && 'raw_code' in job && job.raw_code}
									<div class="text-xs">
										<HighlightCode lines language={job.language} code={job.raw_code} />
									</div>
								{:else if job}
									No code is available
								{:else}
									<Skeleton layout={[[5]]} />
								{/if}
							{:else if viewTab == 'stats'}
								<div class="w-full">
									<MemoryFootprintViewer jobId={job.id} bind:jobUpdateLastFetch />
								</div>
							{:else if job !== undefined && 'result' in job && job.result !== undefined}
								<DisplayResult
									workspaceId={job?.workspace_id}
									jobId={job?.id}
									result={job.result}
									language={job?.language}
									isTest={false}
								/>
							{:else if job}
								No output is available yet
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{:else if !job?.['deleted']}
			<div class="mt-10"></div>
			<FlowProgressBar
				{job}
				bind:currentSubJobProgress={scriptProgress}
				class="py-4 max-w-7xl mx-auto px-4"
			/>
			<div class="w-full mt-10">
				{#if job?.id}
					<FlowStatusViewer
						jobId={job?.id ?? ''}
						on:jobsLoaded={({ detail }) => {
							job = detail
						}}
						on:done={(e) => {
							job = e.detail
						}}
						initialJob={job}
						workspaceId={$workspaceStore}
						bind:selectedJobStep
					/>
				{:else}
					<Skeleton layout={[[5]]} />
				{/if}
			</div>
		{/if}
	</div>
{/if}

<FlowAssetsHandler
	modules={job?.raw_flow?.modules ?? []}
	enableDbExplore
	enablePathScriptAndFlowAssets
/>
