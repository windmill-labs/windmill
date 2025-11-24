<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

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
		isScriptPreview
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
		GitBranch,
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
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/TabsV2.svelte'
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
	import RunBadges from '$lib/components/runs/RunBadges.svelte'
	import { twMerge } from 'tailwind-merge'
	let job: (Job & { result?: any; result_stream?: string }) | undefined = $state()
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
	let jobLoader: JobLoader | undefined = $state(undefined)

	let persistentScriptDrawer: PersistentScriptDrawer | undefined = $state(undefined)

	let showExplicitProgressTip: boolean = $state(
		(localStorage.getItem('hideExplicitProgressTip') ?? 'false') == 'false'
	)

	let lastJobId: string | undefined = $state(undefined)
	let concurrencyKey: string | undefined = $state(undefined)

	let manuallySetLogs: boolean = $state(false)

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
		if (job && (job.result || job.result_stream)) {
			viewTab = 'result'
		} else if (viewTab == 'result') {
			viewTab = 'logs'
		}
	}

	async function getJob() {
		await jobLoader?.watchJob(page.params.run ?? '', {
			change(job: Job & { result_stream?: string }) {
				if (!manuallySetLogs && viewTab == 'logs' && job.result_stream) {
					viewTab = 'result'
				}
			},
			done(job) {
				if (job?.['result'] != undefined) {
					viewTab = 'result'
				}
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
			$initialArgsStore = job?.args
			const state = {
				flow: { value: job?.raw_flow },
				path: job?.script_path + '_fork'
			}
			const encodedArgs = encodeState(job?.args)
			window.open(`/flows/add?initial_args=${encodedArgs}#${encodeState(state)}`)
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
			if (job?.job_kind == 'script' || job?.job_kind == 'flow') {
				let id

				if (job?.job_kind == 'script') {
					id = await JobService.runScriptByHash({
						...commonArgs,
						hash: job.script_hash!,
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
					<Button href="{base}/runs" unifiedSize="md" variant="default">Go to runs page</Button>
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
							<Button nonCaptureEvent variant="default" size="sm" startIcon={{ icon: Trash }} />
						{/snippet}
					</Dropdown>
					{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
						<Button href={runsHref} variant="default" size="sm" startIcon={{ icon: List }}>
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
			{#if job?.type === 'CompletedJob' && job?.job_kind === 'flow' && selectedJobStep !== undefined && selectedJobStepIsTopLevel}
				{#if selectedJobStepType == 'single'}
					<Button
						title={`Re-start this flow from step ${selectedJobStep} (included). ${
							!$enterpriseLicense ? ' This is a feature only available in enterprise edition.' : ''
						}`}
						variant="default"
						unifiedSize="md"
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
								variant="default"
								unifiedSize="md"
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
								<div class="pb-1 text-xs font-semibold text-emphasis"
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
										unifiedSize="md"
										variant="accent"
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
							size="sm"
							startIcon={{ icon: Pen }}>Edit</Button
						>
					{/if}
				{/if}
				<Button
					href={viewHref}
					unifiedSize="md"
					variant="accent"
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
		<div
			class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-6 max-w-7xl mx-auto px-4"
		>
			<div class="flex flex-row flex-wrap gap-6 items-center">
				{#if job}
					{#if 'success' in job && job.success}
						{#if job.is_skipped}
							<FastForward class="text-green-600" size={20} />
						{:else}
							<CheckCircle2 class="text-green-600" size={20} />
						{/if}
					{:else if job && 'success' in job}
						<XCircle class="text-red-700" size={20} />
					{:else if job && 'running' in job && job.running && job.suspend}
						<Hourglass class="text-violet-500" size={20} />
					{:else if job && 'running' in job && job.running}
						<Circle class="text-yellow-500 fill-current" size={20} />
					{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
						<Calendar class="text-secondary" size={20} />
					{:else if job && 'running' in job && job.scheduled_for}
						<Hourglass class="text-primary" size={20} />
					{/if}
					<span class="text-emphasis text-2xl font-semibold"
						>{job.script_path ??
							(job.job_kind == 'dependencies' ? 'lock dependencies' : 'No path')}</span
					>
					<div class="flex flex-row gap-2 items-center flex-wrap">
						<RunBadges
							{job}
							displayPersistentScriptDefinition={!!persistentScriptDefinition}
							openPersistentScriptDrawer={() => {
								persistentScriptDrawer?.open?.(persistentScriptDefinition)
							}}
							{concurrencyKey}
							verySmall={false}
						/>
					</div>
				{/if}
			</div>
		</div>
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
				<!-- Logs and outputs-->
				<div class="mr-2 sm:mr-0 mt-12">
					<Tabs
						bind:selected={viewTab}
						onTabClick={(value) => {
							manuallySetLogs = value == 'logs'
						}}
					>
						<Tab value="result" label="Result" />
						<Tab value="logs" label="Logs" />
						<Tab value="stats" label="Metrics" />
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
							{:else if job !== undefined && (job.result_stream || (job.type == 'CompletedJob' && job.result !== undefined))}
								<DisplayResult
									workspaceId={job?.workspace_id}
									result_stream={job.result_stream}
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
						onJobsLoaded={({ job: newJob }) => {
							job = newJob
						}}
						onDone={({ job: newJob }) => {
							job = newJob
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
