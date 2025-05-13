<script lang="ts">
	import { type Job } from '$lib/gen'
	import { base } from '$lib/base'
	import JobStatus from '$lib/components/JobStatus.svelte'
	import { displayDate, truncateRev } from '$lib/utils'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import TimeAgo from './TimeAgo.svelte'
	import { workspaceStore } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'
	import { Clock, MemoryStick, Calendar, Bot, User, Code2 } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	export let job: Job
	const SMALL_ICON_SIZE = 14
	export let scheduleEditor: ScheduleEditor
</script>

<div
	class="rounded-md p-3 bg-surface-secondary shadow-sm sm:text-sm md:text-base overflow-x-auto"
	style="min-height: 150px;"
>
	<JobStatus {job} />
	<div class="flex flex-row gap-2 items-center">
		<Clock size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
		<span class="text-2xs text-secondary whitespace-nowrap">
			{#if job['success'] != undefined}
				Received job: {displayDate(job.created_at ?? '')}
			{:else}
				Received job <TimeAgo date={job.created_at ?? ''} />
			{/if}
			<Tooltip small>{job?.created_at}</Tooltip>
		</span>
	</div>
	{#if job && 'started_at' in job && job.started_at}
		<div class="flex flex-row gap-2 items-center text-sm">
			<Clock size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
			<span class="whitespace-nowrap">
				Started <TimeAgo agoOnlyIfRecent date={job.started_at ?? ''} />
				<Tooltip small>{job?.started_at}</Tooltip>
			</span>
		</div>
	{/if}
	{#if job && job['mem_peak']}
		<div class="flex flex-row gap-2 items-center text-sm">
			<MemoryStick size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
			<span> Mem peak: {(job['mem_peak'] / 1024).toPrecision(5)}MB</span>
		</div>
	{/if}
	<div>
		{#if job && job.parent_job}
			{#if job.is_flow_step}
				<div class="flex flex-row gap-2 items-center text-sm">
					<BarsStaggered size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
					<span class="whitespace-nowrap text-sm">
						Step of flow
						<a href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}>
							{truncateRev(job.parent_job, 18)}
						</a>
					</span>
				</div>
			{:else}
				<div class="flex flex-row gap-2 items-center text-sm">
					<Bot size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
					<span class="whitespace-nowrap">
						Triggered by parent
						<a href={`${base}/run/${job.parent_job}?workspace=${$workspaceStore}`}>
							{job.parent_job}</a
						>
					</span>
				</div>
			{/if}
		{:else if job && job.schedule_path}
			<div class="flex flex-row gap-2 w-full items-center text-sm">
				<Calendar size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
				<span class="whitespace-nowrap">
					Schedule:
					<!-- svelte-ignore a11y-invalid-attribute -->
					<a
						href="#"
						class="break-words text-blue-600 font-normal"
						on:click={() =>
							scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
					>
						{truncateRev(job.schedule_path, 40)}
					</a>
				</span>
			</div>
		{/if}

		{#if (job && job.job_kind == 'flow') || job?.job_kind == 'script'}
			{@const stem = `${job?.job_kind}s`}
			{@const isScript = job?.job_kind === 'script'}
			{@const viewHref = `${base}/${stem}/get/${isScript ? job?.script_hash : job?.script_path}`}
			<div class="flex flex-row gap-2 items-center">
				{#if isScript}
					<Code2 size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />
				{:else}
					<BarsStaggered size={SMALL_ICON_SIZE} class="text-secondary" />
				{/if}
				<span class="truncate text-sm">
					<a href={viewHref}>{isScript ? job?.script_hash : job?.script_path}</a>
				</span>
			</div>
		{/if}

		<div class="flex items-center gap-2 text-sm">
			<User size={SMALL_ICON_SIZE} class="text-secondary min-w-3.5" />

			<span>
				By {truncateRev(
					job.created_by ?? 'unknown',
					40
				)}{#if (job?.created_by?.length ?? 0) > 40}<Tooltip small
						><div class="break-all">{job.created_by}</div></Tooltip
					>{/if}
				{#if job.permissioned_as !== `u/${job.created_by}` && job.permissioned_as != job.created_by}
					but permissioned as {job.permissioned_as}
				{/if}
			</span>
		</div>
	</div>
	<div class="text-secondary text-2xs pt-2 whitespace-nowrap">
		run id:
		<a href={`${base}/run/${job.id}?workspace=${job.workspace_id}`}> {job.id} </a>
	</div>
</div>
