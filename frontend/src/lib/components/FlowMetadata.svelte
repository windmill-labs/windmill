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
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { onDestroy, onMount } from 'svelte'

	let time = Date.now()
	let interval
	onMount(() => {
		interval = setInterval(() => {
			time = Date.now()
		}, 1000)
	})

	onDestroy(() => {
		interval && clearInterval(interval)
	})

	export let job: Job
	const SMALL_ICON_SCALE = 0.7
	let scheduleEditor: ScheduleEditor
</script>

<ScheduleEditor bind:this={scheduleEditor} />

<div class="rounded-md p-3 bg-gray-50 shadow-sm sm:text-sm md:text-base" style="min-height: 150px;">
	<JobStatus {job} />
	<div>
		<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span
			class="mx-2 text-2xs text-gray-600"
		>
			{#key time}
				Received job {displayDaysAgo(job.created_at ?? '')}
			{/key}</span
		>
	</div>
	{#if job && 'started_at' in job && job.started_at}
		<div>
			<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span class="mx-2">
				{#key time}
					Started {displayDaysAgo(job.started_at ?? '')}{/key}</span
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
				>Triggered by the schedule: <button
					class="break-words text-sm text-blue-600 font-normal"
					on:click={() => scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
					>{job.schedule_path}</button
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
