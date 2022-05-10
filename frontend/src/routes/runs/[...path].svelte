<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import { JobService, Job, CompletedJob } from '../../gen'
	import { displayDate, displayDaysAgo, forLater, truncateHash } from '../../utils'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'
	import {
		faCalendar,
		faCircle,
		faClock,
		faHourglassHalf,
		faRobot,
		faTimes,
		faUser,
		faWind
	} from '@fortawesome/free-solid-svg-icons'
	import { page } from '$app/stores'
	import { sendUserToast } from '../../utils'
	import { goto } from '$app/navigation'
	import PageHeader from '../components/PageHeader.svelte'
	import { workspaceStore } from '../../stores'
	import CenteredPage from '../components/CenteredPage.svelte'
	import Tabs from '../components/Tabs.svelte'

	let jobs: Job[] | undefined
	let error: Error | undefined
	let intervalId: NodeJS.Timer | undefined
	let path: string = $page.params.path
	let createdBefore: string | undefined = $page.url.searchParams.get('createdBefore') ?? undefined
	let showOlderJobs = true
	// The API returns 30 jobs per page. We use it to display a next page button or not.
	const jobsPerPage = 30

	let jobKinds: string

	$: jobKinds =
		$page.url.searchParams.get('jobKinds') ??
		`${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW}`

	$: jobKinds && $workspaceStore && loadJobs()

	const SMALL_ICON_SCALE = 0.7

	async function fetchJobs(
		createdBefore: string | undefined,
		createdAfter?: string
	): Promise<Job[]> {
		return JobService.listJobs({
			workspace: $workspaceStore!,
			createdBefore,
			createdAfter,
			scriptPathExact: path === '' ? undefined : path,
			jobKinds
		})
	}

	async function fetchCompletedJobs(createdBefore: string): Promise<CompletedJob[]> {
		return JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			createdBefore,
			scriptPathExact: path === '' ? undefined : path,
			jobKinds: jobKinds
		})
	}

	async function loadJobs(): Promise<void> {
		try {
			const newJobs = await fetchJobs(createdBefore)
			showOlderJobs = newJobs.length === jobsPerPage
			jobs = newJobs
		} catch (err) {
			sendUserToast(`There was a problem fetching jobs: ${err}`, true)
			error = err
			console.error(JSON.stringify(err))
		}
	}

	async function loadOlderJobs() {
		if (jobs) {
			const ts = jobs[jobs.length - 1].created_at
			const olderJobs = await fetchCompletedJobs(ts!)
			showOlderJobs = olderJobs.length === jobsPerPage
			jobs = jobs.concat(...olderJobs)
		}
	}

	async function syncer() {
		if (jobs && createdBefore === undefined) {
			const reversedJobs = jobs.slice(0, jobsPerPage).reverse()
			const lastIndex = reversedJobs.findIndex((x) => x.type == Job.type.QUEUED_JOB) - 1
			const ts = lastIndex >= 0 ? reversedJobs[lastIndex].created_at : undefined
			const newJobs = await fetchJobs(undefined, ts)
			const oldJobs = jobs.map((x) => x.id)
			jobs = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
			newJobs
				.filter((x) => oldJobs.includes(x.id))
				.forEach((x) => (jobs![jobs?.findIndex((y) => y.id == x.id)!] = x))
			jobs = jobs
		}
	}

	$: {
		if ($workspaceStore) {
			loadJobs()
			intervalId = setInterval(syncer, 5000)
		}
	}

	onDestroy(() => {
		if (intervalId) {
			clearInterval(intervalId)
		}
	})
</script>

<CenteredPage>
	<PageHeader
		title="Runs {path ? `of ${path}` : ''}"
		tooltip="Below is NOT all the runs that have ever been run. You only see the runs whose execution has
been permissioned with privilege that you are allowed to see. In most cases, it will only be
your personal runs, scheduled runs of groups that you are member of, and runs that are
permissioned at the group level of a group you are a member of. Hence, you can safely run
script with sensitive logs knowing that only the users with the relevant permissions would see
it. The permission of a run constraint the ephemeral bearer token that is passed to at
execution time of that run. This is why runs with less permissions are less sensitive because
the bearer token they use has less privilege."
	/>

	<div class="max-w-7x">
		<div class="xl:max-w-screen-lg">
			<Tabs
				tabs={[
					['script,dependencies,preview,flow', 'all'],
					[`${CompletedJob.job_kind.SCRIPT},${CompletedJob.job_kind.FLOW}`, 'runs'],
					[`${CompletedJob.job_kind.FLOW},${CompletedJob.job_kind.FLOWPREVIEW}`, 'previews'],
					[CompletedJob.job_kind.DEPENDENCIES, 'dependencies']
				]}
				dflt={1}
				on:update={(tab) => {
					goto(`?jobKinds=${tab.detail}`)
				}}
			/>
			{#if jobs}
				{#each jobs as job}
					<div class="border rounded">
						<div class="grid grid-cols-2">
							<div class="flex-col py-2">
								<div class="flex flex-row text-sm">
									{#if job === undefined}
										No job found
									{:else}
										<div class="block text-center align-middle pb-3 pt-2 px-6">
											{#if 'success' in job && job.success}
												<Icon
													class="text-green-600"
													data={check}
													scale={SMALL_ICON_SCALE}
													label="Job completed successfully"
												/>
											{:else if 'success' in job}
												<Icon
													class="text-red-700"
													data={faTimes}
													scale={SMALL_ICON_SCALE}
													label="Job completed with an error"
												/>
											{:else if 'running' in job && job.running}
												<Icon
													class="text-yellow-500"
													data={faCircle}
													scale={SMALL_ICON_SCALE}
													label="Job is running"
												/>
											{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
												<Icon
													class="text-gray-700"
													data={faCalendar}
													scale={SMALL_ICON_SCALE}
													label="Job is scheduled to run at a later time"
												/>
											{:else}
												<Icon
													class="text-gray-500"
													data={faHourglassHalf}
													scale={SMALL_ICON_SCALE}
													label="Job is waiting for an executor"
												/>
											{/if}
										</div>

										<h3 class="break-all text-center align-middle py-2">
											{#if job.script_path}
												<a class="pr-3" href="/run/{job.id}">{job.script_path} </a>
											{:else if 'job_kind' in job && job.job_kind == 'preview'}
												<a class="pr-3" href="/run/{job.id}">Preview without path </a>
											{:else if 'job_kind' in job && job.job_kind != 'script'}
												<a class="pr-3" href="/run/{job.id}">lock dependencies</a>
											{/if}
											{#if job.script_hash}
												<a href="/scripts/get/{job.script_hash}" class="commit-hash"
													>{truncateHash(job.script_hash ?? '')}</a
												>
											{/if}
											{#if 'job_kind' in job && job.job_kind != 'script'}<span
													class="bg-blue-200 text-gray-700 text-xs rounded px-1 mx-3 whitespace-nowrap"
													><a href="/run/{job.id}">{job.job_kind}</a></span
												>
											{:else if job.is_flow_step}
												<span class="bg-blue-200 text-gray-700 text-xs rounded px-1 mx-3"
													><a href="/run/{job.parent_job}">step of flow</a></span
												>
											{/if}
										</h3>
									{/if}
								</div>
								<div>
									<span class="pl-14 italic text-gray-500 text-2xs">Run {job.id}</span>
								</div>
							</div>
							<div
								class="px-14 md:pl-40 py-6  text-gray-500 text-xs text-left place-self-start flex flex-col gap-1"
							>
								<div>
									<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
										class="mx-2"
									>
										Created {displayDaysAgo(job.created_at ?? '')}</span
									>
								</div>
								{#if 'started_at' in job && job.started_at}
									<div>
										<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
											class="mx-2"
										>
											Started {displayDaysAgo(job.started_at ?? '')}</span
										>
									</div>
								{/if}
								{#if job && 'duration' in job && job.duration != undefined}
									<div>
										<Icon
											class="text-gray-700"
											data={faHourglassHalf}
											scale={SMALL_ICON_SCALE}
										/><span class="mx-2"> Ran in {job.duration}s</span>
									</div>
								{/if}
								<div>
									{#if job && job.schedule_path}
										<Icon class="text-gray-700" data={faCalendar} scale={SMALL_ICON_SCALE} />
										<span class="mx-2"
											>Triggered by the schedule: <a
												href={`/schedule/add?edit=${job.schedule_path}`}>{job.schedule_path}</a
											></span
										>
									{:else if job && job.parent_job}
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
												Triggered by parent <a href={`/run/${job.parent_job}`}>{job.parent_job}</a
												></span
											>
										{/if}
									{:else}
										<Icon class="text-gray-700" data={faUser} scale={SMALL_ICON_SCALE} /><span
											class="mx-2"
										>
											By {job.created_by}</span
										>
									{/if}
								</div>
								{#if 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
									<div>
										<Icon class="text-gray-700" data={faCalendar} scale={SMALL_ICON_SCALE} /><span
											class="mx-2"
										>
											<span class="bg-blue-200 text-gray-700 text-xs rounded px-1 ">Scheduled</span>
											for {displayDate(job.scheduled_for ?? '')}
										</span>
									</div>
								{:else if 'scheduled_for' in job && !job.running}
									<div>
										<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
											class="mx-2"
										>
											<span class="bg-blue-200 text-gray-700 text-xs rounded px-1 "
												>Waiting for an executor</span
											>
										</span>
									</div>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			{:else if !jobs}
				<div class="text-gray-700">Loading...</div>
			{/if}
		</div>
		{#if error}
			<div>
				{JSON.stringify(error)}
			</div>
		{/if}
	</div>

	<div class="text-center pb-6">
		{#if jobs && jobs.length >= jobsPerPage && showOlderJobs}
			<button class=" mt-4 text-blue-500 text-center text-sm" on:click={loadOlderJobs}>
				Load older jobs</button
			>
		{/if}
	</div>
</CenteredPage>
