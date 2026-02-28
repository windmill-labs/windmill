<script lang="ts">
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
		emptyString,
		encodeState,
		isFlowPreview,
		isNotFlow,
		isScriptPreview
	} from '$lib/utils'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import {
		Activity,
		Calendar,
		List,
		Pen,
		RefreshCw,
		TimerOff,
		Trash,
		Code2,
		ClipboardCopy,
		GitBranch,
		GitFork,
		EllipsisVertical
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
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { ActionRow, Button, Skeleton, Tab, Alert, DrawerContent } from '$lib/components/common'
	import JobDetailHeader from '$lib/components/runs/JobDetailHeader.svelte'
	import FlowExecutionStatus from '$lib/components/runs/FlowExecutionStatus.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/TabsV2.svelte'
	import { goto } from '$lib/navigation'
	import { sendUserToast } from '$lib/toast'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import Portal from '$lib/components/Portal.svelte'

	import MemoryFootprintViewer from '$lib/components/MemoryFootprintViewer.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkflowTimeline from '$lib/components/WorkflowTimeline.svelte'

	import HighlightTheme from '$lib/components/HighlightTheme.svelte'

	import ExecutionDuration from '$lib/components/ExecutionDuration.svelte'
	import { isWindmillTooBigObject } from '$lib/components/job_args'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import { setContext, untrack } from 'svelte'

	import FlowAssetsHandler, {
		initFlowGraphAssetsCtx
	} from '$lib/components/flows/FlowAssetsHandler.svelte'
	import JobAssetsViewer from '$lib/components/assets/JobAssetsViewer.svelte'
	import { page } from '$app/state'
	import { twMerge } from 'tailwind-merge'
	import FlowRestartButton from '$lib/components/FlowRestartButton.svelte'
	import JobOtelTraces from '$lib/components/JobOtelTraces.svelte'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { buildForkEditUrl } from '$lib/utils/editInFork'
	let job: (Job & { result?: any; result_stream?: string }) | undefined = $state()
	let jobUpdateLastFetch: Date | undefined = $state()

	let scriptProgress: number | undefined = $state(undefined)
	let currentJobIsLongRunning: boolean = $state(false)

	let viewTab: 'logs' | 'code' | 'stats' | 'assets' | 'traces' = $state('logs')
	let selectedJobStep: string | undefined = $state(undefined)

	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let restartBranchNames: [number, string][] = []

	let testIsLoading = $state(false)
	let jobLoader: JobLoader | undefined = $state(undefined)

	// Flow execution status state
	let suspendStatus: import('$lib/utils').StateStore<Record<string, { job: Job; nb: number }>> =
		$state({ val: {} })
	let isOwner: boolean = $state(false)

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

	// Initialize view tab to logs since result is now outside tabs
	function initView(): void {
		// Result is now displayed outside tabs, so always default to logs
		viewTab = 'logs'
	}

	async function getJob() {
		await jobLoader?.watchJob(page.params.run ?? '', {
			change(job: Job & { result_stream?: string }) {
				// Result is now displayed outside tabs, no need to switch tabs
			},
			done(job) {
				// Result is now displayed outside tabs, no need to switch tabs
			}
		})
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
		if (
			job &&
			job.type == 'CompletedJob' &&
			(job.job_kind == 'script' || isScriptPreview(job.job_kind))
		) {
			// If error occurred and job is completed
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
			const state = {
				flow: { value: job?.raw_flow },
				path: job?.script_path + '_fork',
				initialArgs: job?.args
			}
			try {
				localStorage.setItem('fork_flow', JSON.stringify(state))
			} catch {
				// Flow too large for localStorage, pass via window reference
				;(window as any).__forkPreviewData = state
			}
			window.open('/flows/add?fork=true')
		} else {
			$initialArgsStore = job?.args
			let n: NewScript = {
				path: job?.script_path + '_fork',
				summary: 'Fork of preview of ' + job?.script_path,
				language: job?.language as NewScript['language'],
				description: '',
				content: job?.raw_code ?? '',
				kind: 'script'
			}
			const encodedArgs = encodeState(job?.args)
			window.open(`/scripts/add?initial_args=${encodedArgs}#${encodeState(n)}`)
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
			if (job?.job_kind == 'script' || job?.job_kind == 'script_hub' || job?.job_kind == 'flow') {
				let id

				if (job?.job_kind == 'script') {
					id = await JobService.runScriptByHash({
						...commonArgs,
						hash: job.script_hash!,
						skipPreprocessor: true
					})
				} else if (job?.job_kind == 'script_hub') {
					id = await JobService.runScriptByPath({
						...commonArgs,
						path: job.script_path!,
						skipPreprocessor: true
					})
				} else {
					id = await JobService.runFlowByPath({
						...commonArgs,
						path: job.script_path!,
						skipPreprocessor: true
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

	let showEditButton = $derived(!isRuleActive('DisableDirectDeployment'))

	$effect(() => {
		job?.id && lastJobId !== job.id && untrack(() => getConcurrencyKey(job))
	})
	$effect(() => {
		$workspaceStore && page.params.run && untrack(() => onRunsPageChange())
	})
	$effect(() => {
		$workspaceStore && page.params.run && jobLoader && untrack(() => onRunsPageChangeWithLoader())
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
						unifiedSize="md"
						variant="subtle"
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
	<JobLoader
		bind:scriptProgress
		bind:this={jobLoader}
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
			<h1 class="text-red-400 mt-6 text-2xl font-semibold"
				>Job {page.params.run} not found in {$workspaceStore}</h1
			>
			<h2 class="text-primary text-lg font-semibold">Are you in the right workspace?</h2>
			<div class="flex flex-col gap-2">
				{#each $userWorkspaces as workspace}
					<div>
						<Button
							variant="default"
							unifiedSize="md"
							on:click={() => {
								goto(`/run/${page.params.run}?workspace=${workspace.id}`)
							}}
						>
							See in {workspace.name}
						</Button>
					</div>
				{/each}
				<div>
					<Button href="{base}/runs" unifiedSize="md" variant="accent">Go to runs page</Button>
				</div>
			</div>
		</div>
	</div>
{:else}
	<Skeleton
		class="max-w-7xl p-4 mx-auto w-full"
		loading={!job}
		layout={[
			// 1. Top Action Bar (buttons on right side)
			[
				{ h: 2.5, w: 60 },
				{ h: 2.5, w: 40 }
			],
			1,
			// 2. Job Header
			[{ h: 12, w: 100 }],
			1,
			// 3. Progress Bar
			[{ h: 2, w: 100 }],
			1.5
		]}
	/>
	<ActionRow class="max-w-7xl px-4 mx-auto w-full">
		{#snippet left()}
			<h1 class="text-sm font-semibold text-primary">run/{page.params.run}</h1>
		{/snippet}
		{#snippet right()}
			{@const isScript = job?.job_kind === 'script'}
			{@const runsHref = `/runs/${job?.script_path}${!isScript ? '?jobKind=flow' : ''}`}
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
						<Button
							nonCaptureEvent
							variant="default"
							unifiedSize="md"
							startIcon={{ icon: Trash }}
						/>
					{/snippet}
				</Dropdown>
				{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
					<Button href={runsHref} variant="default" unifiedSize="md" startIcon={{ icon: List }}>
						View runs
					</Button>
				{/if}
			{/if}
			{@const stem = job?.job_kind === 'script_hub' ? '/scripts' : `/${job?.job_kind}s`}
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
							<Button nonCaptureEvent unifiedSize="md" variant="subtle">
								<div class="flex flex-row items-center">
									<EllipsisVertical size={14} />
								</div>
							</Button>
						{/snippet}
					</Dropdown>
				</div>
			{/if}
			{#if isFlowPreview(job?.job_kind) || isScriptPreview(job?.job_kind)}
				<Button
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: GitBranch }}
					on:click={forkPreview}
				>
					Fork {isFlowPreview(job?.job_kind) ? 'flow' : 'code'} preview
				</Button>
			{/if}
			{#if persistentScriptDefinition !== undefined}
				<Button
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: Activity }}
					on:click={() => {
						persistentScriptDrawer?.open?.(persistentScriptDefinition)
					}}
				>
					Current runs
				</Button>
			{/if}
			{#if job && job?.type != 'CompletedJob' && (!job?.schedule_path || job?.['running'] == true)}
				{#if !forceCancel}
					<Button
						unifiedSize="md"
						variant="accent"
						destructive
						startIcon={{ icon: TimerOff }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
								setTimeout(() => {
									forceCancel = true
								}, 3001)
							}
						}}
						title={`Cancel the ${job?.job_kind === 'script' ? 'script' : job?.job_kind === 'flow' ? 'flow' : 'job'}`}
					>
						Cancel
					</Button>
				{:else}
					<Button
						unifiedSize="md"
						variant="accent"
						destructive
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
					unifiedSize="md"
					variant="default"
					on:click={() => {
						if (!job || !job.schedule_path) {
							return
						}
						scheduleEditor?.openEdit(job.schedule_path, job.job_kind == 'flow')
					}}
					startIcon={{ icon: Calendar }}>Edit schedule</Button
				>
			{/if}
			{#if job?.type === 'CompletedJob' && job?.job_kind === 'flow' && selectedJobStep !== undefined && selectedJobStepIsTopLevel && job.id}
					<FlowRestartButton
					jobId={job.id}
					{selectedJobStep}
					{selectedJobStepType}
					{restartBranchNames}
					onRestartComplete={(newJobId) => {
						goto('/run/' + newJobId + '?workspace=' + $workspaceStore)
					}}
					flowPath={job.script_path}
					flowVersionId={job.script_hash ? parseInt(job.script_hash, 16) : undefined}
					disabled={!$enterpriseLicense}
					enterpriseOnly={!$enterpriseLicense}
				/>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'script_hub' || job?.job_kind === 'flow'}
				<Button
					on:click|once={() => {
						goto(viewHref + `#${computeSharableHash(job?.args)}`)
					}}
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: RefreshCw }}
					loading={runImmediatelyLoading}
					dropdownItems={[
						{
							label: 'Run immediately with same args',
							onClick: () => runImmediately()
						}
					]}
				>
					Run again
				</Button>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				{#if !$userStore?.operator}
					{#if canWrite(job?.script_path ?? '', {}, $userStore)}
						<Button
							on:click|once={() => {
								$initialArgsStore = job?.args
								goto(`${stem}/edit/${job?.script_path}${isScript ? `` : `?nodraft=true`}`)
							}}
							unifiedSize="md"
							variant="default"
							disabled={!showEditButton}
							size="sm"
							startIcon={{ icon: Pen }}>Edit</Button
						>
					{/if}
					{#if !showEditButton && !isRuleActive('DisableWorkspaceForking')}
						<Button
							href={buildForkEditUrl(isScript ? 'script' : 'flow', job?.script_path ?? '')}
							unifiedSize="md"
							variant="default"
							size="sm"
							startIcon={{ icon: GitFork }}>Edit in fork</Button
						>
					{/if}
				{/if}
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'script_hub' || job?.job_kind === 'flow'}
				<Button
					href={viewHref}
					unifiedSize="md"
					variant="accent"
					startIcon={{
						icon:
							job?.job_kind === 'script' || job?.job_kind === 'script_hub'
								? Code2
								: job?.job_kind === 'flow'
									? BarsStaggered
									: Code2
					}}
				>
					View {job?.job_kind === 'script_hub' ? 'script' : job?.job_kind}
				</Button>
			{/if}
		{/snippet}
	</ActionRow>
	<div class="w-full pb-8">
		<!-- Flow Detail Header Card -->
		<div class="max-w-7xl mx-auto px-4 py-0">
			<Skeleton loading={!job} layout={[[24]]} />
			{#if job}
				<JobDetailHeader
					{job}
					{scheduleEditor}
					displayPersistentScriptDefinition={!!persistentScriptDefinition}
					openPersistentScriptDrawer={() => {
						persistentScriptDrawer?.open?.(persistentScriptDefinition)
					}}
					{concurrencyKey}
				/>
			{/if}
		</div>
		{#if job?.['deleted']}
			<div class="max-w-7xl mx-auto w-full px-4 mt-6">
				<Alert type="error" title="Deleted">
					The content of this run was deleted (by an admin, no less)
				</Alert>
			</div>
			<div class="my-4"></div>
		{/if}

		<!-- Flow Progress Bar (for flows only) -->
		{#if job?.job_kind === 'flow' || job?.job_kind === 'flowpreview'}
			<div class="max-w-7xl mx-auto w-full px-4 flex flex-col gap-4 mt-2">
				<FlowProgressBar
					{job}
					bind:currentSubJobProgress={scriptProgress}
					class="w-full"
					textPosition="bottom"
					slim
					showStepId
				/>
				{#if suspendStatus}
					<FlowExecutionStatus
						{job}
						{isOwner}
						{suspendStatus}
						workspaceId={job?.workspace_id}
						innerModules={job?.flow_status?.modules}
					/>
				{/if}
			</div>
		{/if}

		<!-- Arguments and actions -->
		<div class="max-w-7xl mx-auto w-full px-4 mt-12">
			<div class="text-xs text-emphasis font-semibold mb-1">Inputs</div>
			<div class="flex flex-col gap-y-6">
				<JobArgs
					workspace={job?.workspace_id ?? $workspaceStore ?? 'no_w'}
					id={job?.id}
					args={job?.args}
				/>
				{#if job && currentJobIsLongRunning && showExplicitProgressTip && !scriptProgress && 'running' in job}
					<Alert
						class="p-1 flex flex-row relative text-center"
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
							class="absolute m-2 top-0 right-0 inline-flex rounded-md bg-surface-secondary text-primary hover:text-primary focus:outline-none"
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
			</div>
		</div>

		{#if isNotFlow(job?.job_kind)}
			{#if ['python3', 'bun', 'deno'].includes(job?.language ?? '') && (job?.job_kind == 'script' || isScriptPreview(job?.job_kind))}
				<ExecutionDuration bind:job bind:longRunning={currentJobIsLongRunning} />
			{/if}
			<div class="max-w-7xl mx-auto w-full px-4 mb-10">
				{#if job?.workflow_as_code_status && job.job_kind !== 'aiagent'}
					<div class="mt-10"></div>
					<WorkflowTimeline
						flow_status={asWorkflowStatus(job.workflow_as_code_status)}
						flowDone={job.type == 'CompletedJob'}
					/>
				{/if}
				{#if scriptProgress}
					<JobProgressBar {job} {scriptProgress} class="py-4" hideStepTitle={true} />
				{/if}

				<!-- Result Section (moved outside tabs) -->
				{#if job}
					<div class="mr-2 sm:mr-0 mt-12 mb-6">
						<h3 class="text-xs font-semibold text-emphasis mb-1">Result</h3>
						<div class="border rounded-md bg-surface-tertiary p-4 overflow-auto max-h-screen">
							{#if job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined)}
								<DisplayResult
									workspaceId={job?.workspace_id}
									result_stream={job.result_stream}
									jobId={job?.id}
									result={job.result}
									language={job?.language}
									isTest={false}
								/>
							{:else}
								<div class="text-secondary text-sm">No output is available yet</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Logs and outputs-->
				<div class="mr-2 sm:mr-0 mt-6">
					<Tabs bind:selected={viewTab}>
						<Tab value="logs" label="Logs" />
						<Tab value="stats" label="Metrics" />
						<Tab value="traces" label="Traces" />
						<Tab value="assets" label="Assets" />
						{#if isScriptPreview(job?.job_kind)}
							<Tab value="code" label="Code" />
						{/if}
					</Tabs>

					<Skeleton loading={!job} layout={[[5]]} />
					{#if job}
						<div
							class={twMerge(
								'flex flex-row border rounded-md p-2 mt-2 overflow-auto min-h-[600px]',
								viewTab == 'logs' ? 'bg-surface-secondary' : 'bg-surface-tertiary'
							)}
						>
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
							{:else if viewTab == 'traces'}
								<div class="w-full">
									<JobOtelTraces jobId={job.id} />
								</div>
							{:else if viewTab == 'code'}
								{#if job && 'raw_code' in job && job.raw_code}
									<div class="text-xs">
										<HighlightCode lines language={job.language} code={job.raw_code} />
									</div>
								{:else if job}
									<span class="text-sm">No code available</span>
								{:else}
									<Skeleton layout={[[5]]} />
								{/if}
							{:else if viewTab == 'stats'}
								<div class="w-full">
									<MemoryFootprintViewer jobId={job.id} bind:jobUpdateLastFetch />
								</div>
							{:else}
								<div class="w-full p-4 text-secondary">Select a tab to view content</div>
							{/if}
						</div>
					{/if}
				</div>
			</div>
		{:else if !job?.['deleted']}
			<div class="mt-10"></div>

			<div class="w-full mt-10">
				{#if job?.id}
					<FlowStatusViewer
						jobId={job?.id ?? ''}
						onJobsLoaded={({ job: newJob }) => {
							job = newJob
						}}
						onDone={({ job: newJob }) => {
							job = newJob
						}}
						initialJob={job}
						workspaceId={$workspaceStore}
						bind:selectedJobStep
						bind:suspendStatus
						bind:isOwner
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
