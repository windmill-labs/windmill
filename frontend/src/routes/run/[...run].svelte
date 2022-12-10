<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Run ${params.run}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, Job } from '$lib/gen'
	import { canWrite, encodeState, forLater, sendUserToast, truncateHash } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import {
		faRefresh,
		faCircle,
		faTimes,
		faTrash,
		faCalendar,
		faTimesCircle,
		faList,
		faEdit,
		faHourglassHalf,
		faScroll,
		faFastForward
	} from '@fortawesome/free-solid-svg-icons'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { superadmin, userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { Button, ActionRow, Skeleton, Tab } from '$lib/components/common'
	import FlowMetadata from '$lib/components/FlowMetadata.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	$: workspace_id = $page.url.searchParams.get('workspace') ?? $workspaceStore
	$: not_same_workspace = workspace_id !== $workspaceStore

	let job: Job | undefined
	const iconScale = 1

	let viewTab: 'result' | 'logs' | 'code' = 'result'

	// Test
	let testIsLoading = false

	let testJobLoader: TestJobLoader

	const SMALL_ICON_SCALE = 0.7

	async function deleteCompletedJob(id: string): Promise<void> {
		await JobService.deleteCompletedJob({ workspace: workspace_id!, id })
		getLogs()
	}

	async function cancelJob(id: string) {
		try {
			await JobService.cancelQueuedJob({ workspace: workspace_id!, id, requestBody: {} })
			sendUserToast(`job ${id} canceled`)
		} catch (err) {
			sendUserToast('could not cancel job', true)
		}
	}

	// If we get results, focus on that tab. Else, focus on logs
	function initView(): void {
		if (job && 'result' in job && job.result !== undefined) {
			viewTab = 'result'
		} else if (viewTab == 'result') {
			viewTab = 'logs'
		}
	}

	async function getLogs() {
		await testJobLoader?.watchJob($page.params.run)
		initView()
	}

	$: {
		if (workspace_id && $page.params.run && testJobLoader) {
			getLogs()
		}
	}
	let notfound = false
</script>

<TestJobLoader
	on:done={() => (viewTab = 'result')}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job
	workspaceOverride={workspace_id}
	bind:notfound
/>

{#if notfound}
	<CenteredPage>
		<div class="flex flex-col gap-6">
			<h1 class="text-red-400 mt-6">Job {$page.params.run} not found in {workspace_id}</h1>
			<h2>Are you in the right workspace?</h2>
			<div class="flex flex-col gap-2">
				{#each $userWorkspaces as workspace}
					<div>
						<Button variant="border" href="/run/{$page.params.run}?workspace={workspace.id}">
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
		class="!max-w-6xl !px-4 sm:!px-6 md:!px-8"
		loading={!job}
		layout={[0.75, [2, 0, 2], 2.25, [{ h: 1.5, w: 40 }]]}
	/>
	{#if job?.job_kind === 'script' || job?.job_kind === 'flow'}
		<ActionRow applyPageWidth>
			<svelte:fragment slot="left">
				{@const isScript = job?.job_kind === 'script'}
				{@const runsHref = `/runs/${job?.script_path}${!isScript ? '?jobKind=flow' : ''}`}
				{#if job && 'deleted' in job && !job?.deleted && ($superadmin || ($userStore?.is_admin ?? false))}
					<Button
						disabled={not_same_workspace}
						variant="border"
						color="red"
						size="xs"
						startIcon={{ icon: faTrash }}
						on:click={() => job?.id && deleteCompletedJob(job.id)}
					>
						Delete
					</Button>
					<Button
						disabled={not_same_workspace}
						href={runsHref}
						variant="border"
						color="blue"
						size="xs"
						startIcon={{ icon: faList }}
					>
						View runs
					</Button>
				{/if}
			</svelte:fragment>
			<svelte:fragment slot="right">
				{@const stem = `/${job?.job_kind}s`}
				{@const isScript = job?.job_kind === 'script'}
				{@const route = isScript ? job?.script_hash : job?.script_path}
				{@const runHref = `${stem}/run/${route}${
					job?.args ? '?args=' + encodeURIComponent(encodeState(job?.args)) : ''
				}`}
				{@const editHref = `${stem}/edit/${route}${
					isScript
						? `?step=2${job?.args ? `&args=${encodeURIComponent(encodeState(job?.args))}` : ''}`
						: `${job?.args ? `?args=${encodeURIComponent(encodeState(job?.args))}` : ''}`
				}`}
				{@const isRunning = job && 'running' in job && job.running}
				{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
				{#if isRunning}
					<Button
						disabled={not_same_workspace}
						color="red"
						size="xs"
						startIcon={{ icon: faTimesCircle }}
						on:click|once={() => {
							if (job?.id) {
								cancelJob(job?.id)
							}
						}}
					>
						Cancel
					</Button>
				{/if}
				{#if not_same_workspace}
					<span class="text-red-500 text-sm"
						>Disabled because job from a different workspace {workspace_id} (current: {$workspaceStore})</span
					>
				{/if}
				<Button
					href={runHref}
					disabled={isRunning || not_same_workspace}
					color="blue"
					size="xs"
					startIcon={{ icon: faRefresh }}>Run again</Button
				>
				{#if canWrite(job?.script_path ?? '', {}, $userStore)}
					<Button
						disabled={not_same_workspace}
						href={editHref}
						color="blue"
						size="xs"
						startIcon={{ icon: faEdit }}>Edit</Button
					>
				{/if}
				<Button
					disabled={not_same_workspace}
					href={viewHref}
					color="blue"
					size="xs"
					startIcon={{ icon: faScroll }}
				>
					View {job?.job_kind}
				</Button>
			</svelte:fragment>
		</ActionRow>
	{/if}
	<CenteredPage>
		<h1 class="flex flex-row flex-wrap justify-between items-center gap-4 py-6">
			<div>
				{#if job}
					{#if 'success' in job && job.success}
						{#if job.is_skipped}
							<Icon
								class="text-green-600"
								data={faFastForward}
								scale={SMALL_ICON_SCALE}
								label="Job completed successfully but was skipped"
							/>
						{:else}
							<Icon
								class="text-green-600"
								data={check}
								scale={SMALL_ICON_SCALE}
								label="Job completed successfully"
							/>
						{/if}
					{:else if job && 'success' in job}
						<Icon
							class="text-red-700"
							data={faTimes}
							scale={iconScale}
							label="Job completed with an error"
						/>
					{:else if job && 'running' in job && job.running}
						<Icon
							class="text-yellow-500"
							data={faCircle}
							scale={iconScale}
							label="Job is running"
						/>
					{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
						<Icon
							class="text-gray-700"
							data={faCalendar}
							scale={iconScale}
							label="Job is scheduled for a later time"
						/>
					{:else if job && 'running' in job && job.scheduled_for}
						<Icon
							class="text-gray-500"
							data={faHourglassHalf}
							scale={iconScale}
							label="Job is waiting for an executor"
						/>
					{/if}
					{job.script_path ?? (job.job_kind == 'dependencies' ? 'lock dependencies' : 'No path')}
					{#if job.script_hash}
						<a href="/scripts/get/{job.script_hash}"
							><Badge color="gray">{truncateHash(job.script_hash)}</Badge></a
						>
					{/if}
					{#if job && 'job_kind' in job}<Badge color="blue">{job.job_kind}</Badge>
					{/if}
				{/if}
			</div>
		</h1>
		{#if job && 'deleted' in job && job?.deleted}
			<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
				<p class="font-bold">Deleted</p>
				<p>The content of this run was deleted (by an admin, no less)</p>
			</div>
		{/if}

		<!-- Arguments and actions -->
		<div class="flex flex-col mr-2 sm:mr-0 sm:grid sm:grid-cols-3 sm:gap-5">
			<div class="col-span-2">
				<JobArgs args={job?.args} />
			</div>
			<div>
				<Skeleton loading={!job} layout={[[9.5]]} />
				{#if job}<FlowMetadata {job} />{/if}
			</div>
		</div>

		{#if job?.job_kind !== 'flow' && job?.job_kind !== 'flowpreview'}
			<!-- Logs and outputs-->
			<div class="mr-2 sm:mr-0 mt-12">
				<Tabs bind:selected={viewTab}>
					<Tab value="result">Result</Tab>
					<Tab value="logs">Logs</Tab>
					{#if job?.job_kind == 'dependencies'}
						<Tab value="code">Dependencies</Tab>
					{:else if job?.job_kind == 'preview'}
						<Tab value="code">Code</Tab>
					{/if}
				</Tabs>

				<Skeleton loading={!job} layout={[[5]]} />
				{#if job}
					<div class="flex flex-row border rounded-md p-2 mt-2 max-h-1/2 overflow-auto">
						{#if viewTab == 'logs'}
							<div class="w-full">
								<LogViewer isLoading={!(job && 'logs' in job && job.logs)} content={job?.logs} />
							</div>
						{:else if viewTab == 'code'}
							{#if job && 'raw_code' in job && job.raw_code}
								<HighlightCode language={job.language} code={job.raw_code} />
							{:else if job}
								No code is available
							{:else}
								<Skeleton layout={[[5]]} />
							{/if}
						{:else if job !== undefined && 'result' in job && job.result !== undefined}
							<DisplayResult result={job.result} />
						{:else if job}
							No output is available yet
						{/if}
					</div>
				{/if}
			</div>
		{:else}
			<div class="mt-10" />
			<FlowProgressBar {job} class="py-4" />
			<div class="w-full mt-10 mb-20">
				<FlowStatusViewer
					jobId={job.id}
					on:jobsLoaded={({ detail }) => {
						job = detail
					}}
				/>
			</div>
		{/if}
	</CenteredPage>
{/if}
