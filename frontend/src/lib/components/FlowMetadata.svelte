<script lang="ts">
	import type { Job } from '$lib/gen'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import { displayDate } from '$lib/utils'
	import ScheduleEditor from './ScheduleEditor.svelte'
	import TimeAgo from './TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'
	import { Clock, MemoryStick, Calendar, Bot, User, Scroll } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	export let job: Job
	const SMALL_ICON_SIZE = 14
	let scheduleEditor: ScheduleEditor
</script>

<ScheduleEditor bind:this={scheduleEditor} />

<div
	class="rounded-md p-3 bg-surface-secondary shadow-sm sm:text-sm md:text-base"
	style="min-height: 150px;"
>
	<JobStatus {job} />
	<div class="flex flex-row gap-2 items-center">
		<Clock size={SMALL_ICON_SIZE} class="text-secondary" />
		<span class="text-2xs text-secondary">
			{#if job['success'] != undefined}
				Received job: {displayDate(job.created_at ?? '')}
			{:else}
				Received job <TimeAgo date={job.created_at ?? ''} />
			{/if}
			<Tooltip>{job?.created_at}</Tooltip>
		</span>
	</div>
	{#if job && 'started_at' in job && job.started_at}
		<div class="flex flex-row gap-2 items-center">
			<Clock size={SMALL_ICON_SIZE} class="text-secondary" />
			<span>
				Started <TimeAgo withDate agoOnlyIfRecent date={job.started_at ?? ''} />
				<Tooltip>{job?.started_at}</Tooltip>
			</span>
		</div>
	{/if}
	{#if job && job['mem_peak']}
		<div class="flex flex-row gap-2 items-center">
			<MemoryStick size={SMALL_ICON_SIZE} class="text-secondary" />
			<span> Mem peak: {(job['mem_peak'] / 1024).toPrecision(5)}MB</span>
		</div>
	{/if}
	<div>
		{#if job && job.parent_job}
			{#if job.is_flow_step}
				<div class="flex flex-row gap-2 items-center">
					<BarsStaggered size={SMALL_ICON_SIZE} class="text-secondary" />
					<span>
						Step of flow
						<a href={`/run/${job.parent_job}?workspace=${$workspaceStore}`}>
							{job.parent_job}
						</a>
					</span>
				</div>
			{:else}
				<div class="flex flex-row gap-2 items-center">
					<Bot size={SMALL_ICON_SIZE} class="text-secondary" />
					<span>
						Triggered by parent
						<a href={`/run/${job.parent_job}?workspace=${$workspaceStore}`}> {job.parent_job}</a>
					</span>
				</div>
			{/if}
		{:else if job && job.schedule_path}
			<div class="flex flex-row gap-2 items-center">
				<Calendar size={SMALL_ICON_SIZE} class="text-secondary" />
				<span>
					Triggered by the schedule:
					<button
						class="break-words text-sm text-blue-600 font-normal"
						on:click={() =>
							scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
					>
						{job.schedule_path}
					</button>
				</span>
			</div>
		{/if}

		{#if (job && job.job_kind == 'flow') || job?.job_kind == 'script'}
			{@const stem = `/${job?.job_kind}s`}
			{@const isScript = job?.job_kind === 'script'}
			{@const viewHref = `${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			<div class="flex flex-row gap-2 items-center">
				<Scroll size={SMALL_ICON_SIZE} class="text-secondary" />
				<span>
					<a href={viewHref}>{isScript ? job?.script_hash : job?.script_path}</a>
				</span>
			</div>
		{/if}

		<div class="flex flex-row gap-2 items-center">
			<User size={SMALL_ICON_SIZE} class="text-secondary" />

			<span>
				By {job.created_by}
				{#if job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as != job.created_by}
					but permissioned as {job.permissioned_as}
				{/if}
			</span>
		</div>
	</div>
	<div class="text-secondary text-2xs pt-2">
		run id:
		<a href={`/run/${job.id}?workspace=${job.workspace_id}`}> {job.id} </a>
	</div>
</div>
