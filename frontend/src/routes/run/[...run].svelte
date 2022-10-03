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
	import {
		canWrite,
		displayDaysAgo,
		encodeState,
		forLater,
		sendUserToast,
		truncateHash
	} from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import {
		faBolt,
		faCircle,
		faTimes,
		faTrash,
		faCalendar,
		faTimesCircle,
		faClock,
		faUser,
		faList,
		faEdit,
		faHourglassHalf,
		faRobot,
		faScroll,
		faWind,
		faFastForward
	} from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'

	import { userStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import ArgInfo from '$lib/components/ArgInfo.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { Button } from '$lib/components/common'

	let workspace_id_query: string | undefined = $page.url.searchParams.get('workspace') ?? undefined
	let workspace_id: string | undefined

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
		if (job && 'result' in job && job.result) {
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
		if ($workspaceStore && $page.params.run && testJobLoader) {
			workspace_id = workspace_id_query ?? $workspaceStore
			getLogs()
		}
	}
</script>

<TestJobLoader
	on:done={() => (viewTab = 'result')}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job
/>

<CenteredPage>
	<div class="flex flex-row flex-wrap justify-between items-center gap-4 pb-4">
		<h1>
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
						<a
							href="/scripts/get/{job.script_hash}"
							class="text-2xs text-gray-500 bg-gray-100 font-mono"
							>{truncateHash(job.script_hash)}</a
						>
					{:else if job && 'job_kind' in job}<span
							class="bg-blue-200 text-gray-700 text-xs rounded px-1 mx-3">{job.job_kind}</span
						>{/if}
				{:else}
					<Icon class="text-gray-200" data={faCircle} scale={iconScale} /> Loading...
				{/if}
			</div>
		</h1>
		<div class="flex flex-wrap gap-2">
			{#if job && 'deleted' in job && !job?.deleted && ($userStore?.is_admin ?? false)}
				<Button
					variant="border"
					color="red"
					size="sm"
					startIcon={{ icon: faTrash }}
					on:click={() => {
						if (job?.id) {
							deleteCompletedJob(job?.id)
						}
					}}
				>
					Delete
				</Button>
			{/if}
			{#if job && 'running' in job && job.running}
				<Button
					variant="border"
					color="red"
					size="sm"
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
			{#if job?.job_kind == 'script'}
				{#if canWrite(job?.script_path ?? '', {}, $userStore)}
					<Button
						href="/scripts/edit/{job?.script_hash}?step=2"
						variant="border"
						color="blue"
						size="sm"
						startIcon={{ icon: faEdit }}
					>
						Edit
					</Button>
				{/if}
				<Button
					href="/scripts/get/{job?.script_hash}"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faScroll }}
				>
					View script
				</Button>
				<Button
					href="/runs/{job?.script_path}"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faList }}
				>
					View runs
				</Button>
				<Button
					href="/scripts/run/{job?.script_hash}{job?.args
						? `?args=${encodeURIComponent(encodeState(job?.args))}`
						: ''}"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faBolt }}
				>
					Run again
				</Button>
			{:else if job?.job_kind == 'flow'}
				{#if canWrite(job?.script_path ?? '', {}, $userStore)}
					<Button
						href="/flows/edit/{job?.script_path}"
						variant="border"
						color="blue"
						size="sm"
						startIcon={{ icon: faEdit }}
					>
						Edit
					</Button>
				{/if}
				<Button
					href="/flows/get/{job?.script_path}"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faScroll }}
				>
					View flow
				</Button>
				<Button
					href="/runs/{job?.script_path}?jobKind=flow"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faList }}
				>
					View runs
				</Button>
				<Button
					href="/flows/run/{job?.script_path}{job?.args
						? `?args=${encodeURIComponent(encodeState(job?.args))}`
						: ''}"
					variant="border"
					color="blue"
					size="sm"
					startIcon={{ icon: faBolt }}
				>
					Run again
				</Button>
			{/if}
		</div>
	</div>
	{#if job && 'deleted' in job && job?.deleted}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
			<p class="font-bold">Deleted</p>
			<p>The content of this run was deleted (by an admin, no less)</p>
		</div>
	{/if}

	<!-- Arguments and actions -->
	<div class="flex flex-col mr-2 sm:mr-0 sm:grid sm:grid-cols-3 sm:gap-5">
		<div class="col-span-2">
			<TableCustom class="px-10 py-4">
				<tr slot="header-row"
					><th>Argument</th>
					<th>Value</th></tr
				>
				<tbody slot="body">
					{#if job && job.args && Object.keys(job.args).length > 0}
						{#each Object.entries(job.args) as [arg, value]}
							<tr>
								<td>{arg}</td>
								<td> <ArgInfo {value} /></td>
							</tr>
						{/each}
					{:else if job}
						<tr>No arguments</tr>
					{:else}
						<tr>Loading</tr>
					{/if}
				</tbody>
			</TableCustom>

			{#if job?.job_kind == 'flow' || job?.job_kind == 'flowpreview'}
				<div class="mt-10" />
				<div class="max-w-lg">
					<FlowStatusViewer jobId={job.id} />
				</div>
			{/if}
		</div>
		<div>
			<div
				class="rounded-md p-3 bg-gray-50 shadow-sm sm:text-sm md:text-base"
				style="min-height: 150px;"
			>
				<JobStatus {job} />
				{#if job}
					<div>
						<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
							class="mx-2"
						>
							Created {displayDaysAgo(job.created_at ?? '')}</span
						>
					</div>
					{#if job && 'started_at' in job && job.started_at}
						<div>
							<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
								class="mx-2"
							>
								Started {displayDaysAgo(job.started_at ?? '')}</span
							>
						</div>
					{/if}
					<div>
						{#if job && job.parent_job}
							{#if job.is_flow_step}
								<Icon class="text-gray-700" data={faWind} scale={SMALL_ICON_SCALE} /><span
									class="mx-2"
								>
									Step of flow <a href={`/run/${job.parent_job}`}>{job.parent_job}</a></span
								>
							{:else}
								<Icon class="text-gray-700" data={faRobot} scale={SMALL_ICON_SCALE} /><span
									class="mx-2"
								>
									Triggered by parent <a href={`/run/${job.parent_job}`}>{job.parent_job}</a></span
								>
							{/if}
						{:else if job && job.schedule_path}
							<Icon class="text-gray-700" data={faCalendar} scale={SMALL_ICON_SCALE} />
							<span class="mx-2"
								>Triggered by the schedule: <a
									href={`/schedule/add?edit=${job.schedule_path}&isFlow=${job.job_kind == 'flow'}`}
									>{job.schedule_path}</a
								></span
							>
						{/if}
						<div>
							<Icon class="text-gray-700" data={faUser} scale={SMALL_ICON_SCALE} /><span
								class="mx-2"
							>
								By {job.created_by}
								{#if job.permissioned_as != `u/${job.created_by}`}but permissioned as {job.permissioned_as}{/if}
							</span>
						</div>
					</div>
					<div class="text-gray-700 text-2xs pt-2">
						run id: <a href={`/run/${job.id}`}>{job.id}</a>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#if job?.job_kind != 'flow' && job?.job_kind != 'flowpreview'}
		<!-- Logs and outputs-->
		<div class="mr-2 sm:mr-0 mt-12">
			<div class="flex flex-col sm:flex-row text-base">
				<button
					class=" py-1 px-6 block border-gray-200 hover:bg-gray-50  {viewTab != 'result'
						? 'text-gray-500'
						: 'text-gray-700 font-semibold  '}"
					on:click={() => (viewTab = 'result')}
				>
					Result <Tooltip
						>What is returned by the <span class="font-mono">main</span> function of the script,
						stringified to JSON. Then for some specific cases, like having "png", "jpeg" or "file"
						as sole key, they are displayed more richly. See
						<a href="https://docs.windmill.dev/docs/reference#rich-display-rendering">here</a> for more
						details.</Tooltip
					>
				</button>
				<button
					class="py-1 px-6 block border-gray-200 hover:bg-gray-50  {viewTab != 'logs'
						? 'text-gray-500'
						: 'text-gray-700 font-semibold  '}"
					on:click={() => (viewTab = 'logs')}
				>
					Logs
				</button>
				{#if job && 'raw_code' in job && job.raw_code}
					<button
						class="py-1 px-6 block border-gray-200 hover:bg-gray-50  {viewTab != 'code'
							? 'text-gray-500'
							: 'text-gray-700 font-semibold  '}"
						on:click={() => (viewTab = 'code')}
					>
						{job.job_kind == 'dependencies' ? 'Input Dependencies' : 'Code previewed'}
					</button>
				{/if}
			</div>
			<div class="flex flex-row border rounded-md p-3 max-h-1/2 overflow-auto">
				{#if viewTab == 'logs'}
					<div class="w-full">
						<LogViewer isLoading={!(job && 'logs' in job && job.logs)} content={job?.logs} />
					</div>
				{:else if viewTab == 'code'}
					{#if job && 'raw_code' in job && job.raw_code}
						<HighlightCode language={job.language} code={job.raw_code} />
					{:else if job}No code is available
					{:else}Loading...{/if}
				{:else if job && 'result' in job && job.result}<DisplayResult result={job.result} />
				{:else if job}No output is available yet
				{:else}Loading...
				{/if}
			</div>
		</div>
	{/if}
</CenteredPage>
