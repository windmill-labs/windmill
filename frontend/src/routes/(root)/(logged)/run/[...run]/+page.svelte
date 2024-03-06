<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, Job, ScriptService, Script, type WorkflowStatus } from '$lib/gen'
	import { canWrite, copyToClipboard, displayDate, emptyString, truncateHash } from '$lib/utils'
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
		MoreVertical
	} from 'lucide-svelte'

	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import {
		enterpriseLicense,
		runFormStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import {
		ActionRow,
		Button,
		Popup,
		Skeleton,
		Tab,
		Alert,
		MenuItem,
		DrawerContent
	} from '$lib/components/common'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'
	import { forLater } from '$lib/forLater'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import PersistentScriptDrawer from '$lib/components/PersistentScriptDrawer.svelte'
	import Portal from 'svelte-portal'
	import MemoryFootprintViewer from '$lib/components/MemoryFootprintViewer.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import Toggle from '$lib/components/Toggle.svelte'
	import WorkflowTimeline from '$lib/components/WorkflowTimeline.svelte'

	let job: Job | undefined
	let jobUpdateLastFetch: Date | undefined

	let viewTab: 'result' | 'logs' | 'code' | 'stats' = 'result'
	let selectedJobStep: string | undefined = undefined
	let branchOrIterationN: number = 0

	let selectedJobStepIsTopLevel: boolean | undefined = undefined
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = 'single'
	let restartBranchNames: [number, string][] = []

	let testIsLoading = false
	let testJobLoader: TestJobLoader

	let persistentScriptDrawer: PersistentScriptDrawer

	async function deleteCompletedJob(id: string): Promise<void> {
		await JobService.deleteCompletedJob({ workspace: $workspaceStore!, id })
		getLogs()
	}

	async function cancelJob(id: string) {
		try {
			if (forceCancel) {
				await JobService.forceCancelQueuedJob({ workspace: $workspaceStore!, id, requestBody: {} })
				setTimeout(getLogs, 5000)
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

	async function getLogs() {
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

	let persistentScriptDefinition: Script | undefined = undefined

	async function onJobLoaded() {
		if (job === undefined || job.job_kind !== 'script' || job.script_hash === undefined) {
			return
		}
		const script = await ScriptService.getScriptByHash({
			workspace: $workspaceStore!,
			hash: job.script_hash
		})
		if (script.restart_unless_cancelled ?? false) {
			persistentScriptDefinition = script
		}
	}

	$: {
		if ($workspaceStore && $page.params.run && testJobLoader) {
			forceCancel = false
			getLogs()
		}
	}

	$: selectedJobStep !== undefined && onSelectedJobStepChange()
	$: job && onJobLoaded()

	let notfound = false
	let forceCancel = false

	let debugViewer: Drawer
	let debugContent: any = undefined
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

	let redactSensitive = false

	function asWorkflowStatus(x: any): Record<string, WorkflowStatus> {
		return x as Record<string, WorkflowStatus>
	}
</script>

{#if (job?.job_kind == 'flow' || job?.job_kind == 'flowpreview') && job?.['running'] && job?.parent_job == undefined}
	<Drawer bind:this={debugViewer} size="800px">
		<DrawerContent title="Debug Detail" on:close={debugViewer.closeDrawer}>
			<svelte:fragment slot="actions">
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
			</svelte:fragment>
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

<TestJobLoader
	on:done={() => (viewTab = 'result')}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job
	bind:jobUpdateLastFetch
	workspaceOverride={$workspaceStore}
	bind:notfound
/>

<Portal>
	<PersistentScriptDrawer bind:this={persistentScriptDrawer} />
</Portal>

{#if notfound}
	<CenteredPage>
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
					<Button href="/runs">Go to runs page</Button>
				</div>
			</div>
		</div>
	</CenteredPage>
{:else}
	<Skeleton
		class="!max-w-7xl !px-4 sm:!px-6 md:!px-8"
		loading={!job}
		layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }]]}
	/>
	<ActionRow applyPageWidth>
		<svelte:fragment slot="left">
			{@const isScript = job?.job_kind === 'script'}
			{@const runsHref = `/runs/${job?.script_path}${!isScript ? '?jobKind=flow' : ''}`}
			<div class="flex gap-2 items-center">
				{#if job && 'deleted' in job && !job?.deleted && ($superadmin || ($userStore?.is_admin ?? false))}
					<ButtonDropdown target="body" hasPadding={false}>
						<svelte:fragment slot="buttonReplacement">
							<Button nonCaptureEvent variant="border" size="sm" startIcon={{ icon: Trash }} />
						</svelte:fragment>
						<svelte:fragment slot="items">
							<MenuItem
								on:click={() => {
									job?.id && deleteCompletedJob(job.id)
								}}
							>
								<span class="text-red-600"> Delete result, logs and args (admin only) </span>
							</MenuItem>
						</svelte:fragment>
					</ButtonDropdown>
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
		</svelte:fragment>
		<svelte:fragment slot="right">
			{@const stem = `/${job?.job_kind}s`}
			{@const isScript = job?.job_kind === 'script'}
			{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			{#if (job?.job_kind == 'flow' || job?.job_kind == 'flowpreview') && job?.['running'] && job?.parent_job == undefined}
				<div class="inline">
					<ButtonDropdown hasPadding={false}>
						<svelte:fragment slot="buttonReplacement">
							<Button nonCaptureEvent size="xs" color="light">
								<div class="flex flex-row items-center">
									<MoreVertical size={14} />
								</div>
							</Button>
						</svelte:fragment>
						<svelte:fragment slot="items">
							<MenuItem on:click={debugInfo}>Show Flow Debug Info</MenuItem>
						</svelte:fragment>
					</ButtonDropdown>
				</div>
			{/if}
			{#if persistentScriptDefinition !== undefined}
				<Button
					color="blue"
					size="md"
					startIcon={{ icon: Activity }}
					on:click={() => {
						persistentScriptDrawer.open?.(persistentScriptDefinition)
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
					</Button>
				{:else}
					<Popup floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}>
						<svelte:fragment slot="button">
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
						</svelte:fragment>
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
										on:click|stopPropagation={() => {}}
									/>
								{:else}
									<select
										bind:value={branchOrIterationN}
										class="!w-32 grow"
										on:click|stopPropagation={() => {}}
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
					</Popup>
				{/if}
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				<Button
					on:click|once={() => {
						$runFormStore = job?.args
						goto(viewHref)
					}}
					color="blue"
					size="sm"
					startIcon={{ icon: RefreshCw }}>Run again</Button
				>
			{/if}
			{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
				{#if !$userStore?.operator}
					{#if canWrite(job?.script_path ?? '', {}, $userStore)}
						<Button
							on:click|once={() => {
								$runFormStore = job?.args
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
		</svelte:fragment>
	</ActionRow>
	<CenteredPage>
		<h1 class="flex flex-row flex-wrap justify-between items-center gap-x-4 py-6">
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
					{:else if job && 'running' in job && job.running}
						<Circle class="text-yellow-500 fill-current" size={14} />
					{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
						<Calendar class="text-secondary" size={14} />
					{:else if job && 'running' in job && job.scheduled_for}
						<Hourglass class="text-tertiary" size={14} />
					{/if}
					{job.script_path ?? (job.job_kind == 'dependencies' ? 'lock dependencies' : 'No path')}
					<div class="flex flex-row gap-2 items-center">
						{#if job.script_hash}
							<a href="/scripts/get/{job.script_hash}?workspace={$workspaceStore}"
								><Badge color="gray">{truncateHash(job.script_hash)}</Badge></a
							>
						{/if}
						{#if job && 'job_kind' in job}
							<div>
								<Badge color="blue">{job.job_kind}</Badge>
							</div>
						{/if}
						{#if persistentScriptDefinition}
							<button on:click={() => persistentScriptDrawer.open?.(persistentScriptDefinition)}
								><Badge color="red">persistent</Badge></button
							>
						{/if}
						{#if job && 'priority' in job}
							<div>
								<Badge color="red">priority: {job.priority}</Badge>
							</div>
						{/if}
						{#if job.tag && !['deno', 'python3', 'flow', 'other', 'go', 'postgresql', 'mysql', 'bigquery', 'snowflake', 'mssql', 'graphql', 'nativets', 'bash', 'powershell', 'other', 'dependency'].includes(job.tag)}
							<div>
								<Badge color="indigo">Tag: {job.tag}</Badge>
							</div>
						{/if}
						{#if !job.visible_to_owner}<Badge color="red"
								>only visible to you <Tooltip
									>The option to hide this run from the owner of this script or flow was activated</Tooltip
								></Badge
							>
						{/if}
					</div>
				{/if}
			</div>
		</h1>
		{#if job?.['deleted']}
			<Alert type="error" title="Deleted">
				The content of this run was deleted (by an admin, no less)
			</Alert>
			<div class="my-2" />
		{/if}

		<!-- Arguments and actions -->
		<div class="flex flex-col gap-y-8 mr-2 sm:mr-0 sm:grid sm:grid-cols-3 sm:gap-10">
			<div class="col-span-2">
				<JobArgs args={job?.args} />
			</div>
			<div>
				<Skeleton loading={!job} layout={[[9.5]]} />
				{#if job}<FlowMetadata {job} />{/if}
			</div>
		</div>

		{#if job?.['scheduled_for'] && forLater(job?.['scheduled_for'])}
			<h2 class="mt-10">Scheduled to be executed later: {displayDate(job?.['scheduled_for'])}</h2>
			<div class="w-full pt-8">
				<LogViewer
					jobId={job.id}
					isLoading={!(job && 'logs' in job && job.logs)}
					content={job?.logs}
					tag={job?.tag}
				/>
			</div>
		{:else if job?.job_kind !== 'flow' && job?.job_kind !== 'flowpreview' && job?.job_kind !== 'singlescriptflow'}
			{#if job?.flow_status}
				<div class="mt-10" />
				<WorkflowTimeline
					flow_status={asWorkflowStatus(job.flow_status)}
					flowDone={job.type == 'CompletedJob'}
				/>
			{/if}
			<!-- Logs and outputs-->
			<div class="mr-2 sm:mr-0 mt-12">
				<Tabs bind:selected={viewTab}>
					<Tab value="result">Result</Tab>
					<Tab value="logs">Logs</Tab>
					<Tab value="stats">Metrics</Tab>
					{#if job?.job_kind == 'preview'}
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
									isLoading={!(job && 'logs' in job && job.logs)}
									content={job?.logs}
									tag={job?.tag}
								/>
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
							<DisplayResult workspaceId={job?.workspace_id} jobId={job?.id} result={job.result} />
						{:else if job}
							No output is available yet
						{/if}
					</div>
				{/if}
			</div>
		{:else if !job?.['deleted']}
			<div class="mt-10" />
			<FlowProgressBar {job} class="py-4" />
			<div class="w-full mt-10 mb-20">
				<FlowStatusViewer
					jobId={job.id}
					on:jobsLoaded={({ detail }) => {
						job = detail
					}}
					workspaceId={$workspaceStore}
					bind:selectedJobStep
				/>
			</div>
		{/if}
	</CenteredPage>
{/if}
