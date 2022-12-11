<script lang="ts">
	import type { Job } from '$lib/gen'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import Icon from 'svelte-awesome'
	import { displayDaysAgo } from '$lib/utils'
	import {
		faCalendar,
		faClock,
		faRobot,
		faScroll,
		faUser,
		faBarsStaggered
	} from '@fortawesome/free-solid-svg-icons'

	export let job: Job
	const SMALL_ICON_SCALE = 0.7
</script>

<div class="rounded-md p-3 bg-gray-50 shadow-sm sm:text-sm md:text-base" style="min-height: 150px;">
	<JobStatus {job} />
	<div>
		<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span class="mx-2">
			Created {displayDaysAgo(job.created_at ?? '')}</span
		>
	</div>
	{#if job && 'started_at' in job && job.started_at}
		<div>
			<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span class="mx-2">
				Started {displayDaysAgo(job.started_at ?? '')}</span
			>
		</div>
	{/if}
	<div>
		{#if job && job.parent_job}
			{#if job.is_flow_step}
				<Icon class="text-gray-700" data={faBarsStaggered} scale={SMALL_ICON_SCALE} /><span
					class="mx-2"
				>
					Step of flow <a href={`/run/${job.parent_job}`}>{job.parent_job}</a></span
				>
			{:else}
				<Icon class="text-gray-700" data={faRobot} scale={SMALL_ICON_SCALE} /><span class="mx-2">
					Triggered by parent <a href={`/run/${job.parent_job}`}>{job.parent_job}</a></span
				>
			{/if}
		{:else if job && job.schedule_path}
			<Icon class="text-gray-700" data={faCalendar} scale={SMALL_ICON_SCALE} />
			<span
				>Triggered by the schedule: <a
					href={`/schedule/add?edit=${job.schedule_path}&isFlow=${job.job_kind == 'flow'}`}
					>{job.schedule_path}</a
				></span
			>
		{/if}

		{#if (job && job.job_kind == 'flow') || job?.job_kind == 'script'}
			{@const stem = `/${job?.job_kind}s`}
			{@const isScript = job?.job_kind === 'script'}
			{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			<div>
				<Icon class="text-gray-700" data={faScroll} scale={SMALL_ICON_SCALE} /><span class="mx-2">
					<a href={viewHref}>{isScript ? job?.script_hash : job?.script_path}</a>
				</span>
			</div>
		{/if}

		<div>
			<Icon class="text-gray-700" data={faUser} scale={SMALL_ICON_SCALE} /><span class="mx-2">
				By {job.created_by}
				{#if job.permissioned_as !== `u/${job.created_by}`}but permissioned as {job.permissioned_as}{/if}
			</span>
		</div>
	</div>
	<div class="text-gray-700 text-2xs pt-2">
		run id: <a href={`/run/${job.id}?workspace=${job.workspace_id}`}>{job.id}</a>
	</div>
</div>
