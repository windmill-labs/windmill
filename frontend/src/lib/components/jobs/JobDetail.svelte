<script lang="ts">
	import type { Job } from '$lib/gen'
	import {
		displayDate,
		displayDaysAgo,
		forLater,
		msToSec,
		truncateHash,
		truncateRev
	} from '$lib/utils'
	import {
		faCalendar,
		faCircle,
		faClock,
		faFastForward,
		faHourglassHalf,
		faRobot,
		faTimes,
		faUser,
		faWind
	} from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { check } from 'svelte-awesome/icons'

	const SMALL_ICON_SCALE = 0.7

	export let job: Job
</script>

<div class="border rounded py-4">
	<div class="grid grid-cols-1 lg:grid-cols-3 w-full gap-4">
		<div class="flex-col">
			<div class="flex flex-row text-sm">
				{#if job === undefined}
					No job found
				{:else}
					<div class="block text-center align-middle pb-3 pt-2 px-6">
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

					<div class="break-all py-2">
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
					</div>
				{/if}
			</div>
			<div>
				<span class="pl-14 italic text-gray-500 text-2xs  whitespace-nowrap overflow-hidden"
					>Run {job.id}</span
				>
			</div>
		</div>
		<div class="bg-white grid grid-cols-2 gap-x-2 col-span-2">
			<div class="w-full text-gray-500 text-xs text-left flex flex-col gap-1 mx-4 overflow-hidden">
				<div>
					<Icon class="text-gray-700" data={faUser} scale={SMALL_ICON_SCALE} /><span class="mx-2">
						By {job.created_by}</span
					>
				</div>
				{#if job && 'duration_ms' in job && job.duration_ms != undefined}
					<div>
						<Icon class="text-gray-700" data={faHourglassHalf} scale={SMALL_ICON_SCALE} /><span
							class="mx-2"
						>
							Ran in {msToSec(job.duration_ms)}s</span
						>
					</div>
				{/if}
				<div>
					{#if job && job.parent_job}
						{#if job.is_flow_step}
							<Icon class="text-gray-700" data={faWind} scale={SMALL_ICON_SCALE} /><span
								class="mx-2"
							>
								Step of flow <a href={`/run/${job.parent_job}`}>{truncateRev(job.parent_job, 6)}</a
								></span
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
				</div>
			</div>
			<div class="text-gray-500 text-xs text-left place-self-start flex flex-col gap-1">
				<div>
					<Icon class="text-gray-700" data={faClock} scale={SMALL_ICON_SCALE} /><span class="mx-2">
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
</div>
