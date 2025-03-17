<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import type { Job } from '$lib/gen'
	import {
		displayDate,
		msToReadableTime,
		truncateHash,
		truncateRev,
		isJobCancelable,
		isFlowPreview,
		isScriptPreview
	} from '$lib/utils'
	import { Badge, Button } from '../common'
	import ScheduleEditor from '../ScheduleEditor.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import {
		Calendar,
		Check,
		FastForward,
		Folder,
		Hourglass,
		ListFilter,
		Play,
		ShieldQuestion,
		X
	} from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import TimeAgo from '../TimeAgo.svelte'
	import { forLater } from '$lib/forLater'
	import { twMerge } from 'tailwind-merge'
	import Portal from '$lib/components/Portal.svelte'

	import WaitTimeWarning from '../common/waitTimeWarning/WaitTimeWarning.svelte'

	const dispatch = createEventDispatcher()

	export let job: Job
	export let selected: boolean = false
	export let containerWidth: number = 0
	export let containsLabel: boolean = false
	export let activeLabel: string | null
	export let isSelectingJobsToCancel: boolean = false

	let scheduleEditor: ScheduleEditor

	$: isExternal = job && job.id === '-'
</script>

<Portal name="run-row">
	<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />
</Portal>
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={twMerge(
		'hover:bg-surface-hover cursor-pointer',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'flex flex-row items-center h-full'
	)}
	style="width: {containerWidth}px"
	on:click={() => {
		if (!isSelectingJobsToCancel || isJobCancelable(job)) {
			dispatch('select')
		}
	}}
>
	<div class="w-1/12 flex justify-center">
		{#if isSelectingJobsToCancel && isJobCancelable(job)}
			<div class="px-2">
				<input type="checkbox" checked={selected} />
			</div>
		{/if}
		{#if isExternal}
			<Badge color="gray" baseClass="!px-1.5">
				<ShieldQuestion size={14} />
			</Badge>
		{:else if 'success' in job && job.success}
			{#if job.is_skipped}
				<Badge color="green" rounded>
					<FastForward size={14} />
				</Badge>
			{:else}
				<Badge color="green" baseClass="!px-1.5">
					<Check size={14} />
				</Badge>
			{/if}
		{:else if 'success' in job}
			<Badge color="red" baseClass="!px-1.5">
				<X size={14} />
			</Badge>
		{:else if 'running' in job && job.running && job.suspend}
			<Badge color="violet" baseClass="!px-1.5" title="Suspended">
				<Hourglass size={14} />
			</Badge>
		{:else if 'running' in job && job.running}
			<Badge color="yellow" baseClass="!px-1.5">
				<Play size={14} />
			</Badge>
		{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
			<Badge color="blue" baseClass="!px-1.5">
				<Calendar size={14} />
			</Badge>
		{:else if job.canceled}
			<Badge color="red" baseClass="!px-1.5">
				<Hourglass size={14} />
			</Badge>
		{:else}
			<Badge baseClass="!px-1.5">
				<Hourglass size={14} />
			</Badge>
		{/if}
	</div>

	<div class="w-4/12 flex justify-start">
		<div class="flex flex-row items-center gap-1 text-gray-500 dark:text-gray-300 text-2xs">
			{#if job}
				{#if 'started_at' in job && job.started_at}
					Started <TimeAgo agoOnlyIfRecent date={job.started_at ?? ''} />
					{#if job && 'duration_ms' in job && job.duration_ms != undefined}
						(Ran in {msToReadableTime(
							job.duration_ms
						)}{#if job.job_kind == 'flow' || isFlowPreview(job.job_kind)}&nbsp;total{/if})
					{/if}
					{#if job && (job.self_wait_time_ms || job.aggregate_wait_time_ms)}
						<WaitTimeWarning
							self_wait_time_ms={job.self_wait_time_ms}
							aggregate_wait_time_ms={job.aggregate_wait_time_ms}
							variant="icon"
						/>
					{/if}
				{:else if `scheduled_for` in job && job.scheduled_for && forLater(job.scheduled_for)}
					Scheduled for {displayDate(job.scheduled_for)}
				{:else if job.canceled}
					Cancelling job... (created <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />)
				{:else if `scheduled_for` in job && job.scheduled_for && forLater(job.scheduled_for)}
					Waiting for executor (scheduled for <TimeAgo
						agoOnlyIfRecent
						date={job.scheduled_for || ''}
					/>)
				{:else}
					Waiting for executor (created <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />)
				{/if}
			{/if}
		</div>
	</div>

	<div class="w-4/12 flex justify-start flex-col">
		<div class="flex flex-row text-sm">
			{#if job === undefined}
				No job found
			{:else}
				<div class="flex flex-row gap-1 min-w-0">
					<div class="whitespace-nowrap text-xs font-semibold truncate">
						{#if job.script_path}
							<div class="flex flex-row gap-1 items-center">
								{#if isExternal}
									<span class="w-30 justify-center">-</span>
								{:else}
									<a
										href="{base}/run/{job.id}?workspace={job.workspace_id}"
										class="truncate w-30 dark:text-blue-400"
									>
										{job.script_path}
									</a>
									<Button
										title="Filter by path"
										size="xs2"
										color="light"
										on:click={() => {
											dispatch('filterByPath', job.script_path)
										}}
									>
										<ListFilter size={10} />
									</Button>
								{/if}
								{#if job.script_path?.startsWith('f/')}
									<Button
										title="Filter by folder"
										size="xs2"
										color="light"
										on:click={() => {
											// split script_path by / and get the second element
											const folder = job.script_path?.split('/')[1]

											dispatch('filterByFolder', folder)
										}}
									>
										<Folder size={10} />
									</Button>
								{/if}
							</div>
						{:else if 'job_kind' in job && isScriptPreview(job.job_kind)}
							<a href="{base}/run/{job.id}?workspace={job.workspace_id}">Preview without path </a>
						{:else if 'job_kind' in job && job.job_kind == 'dependencies'}
							<a href="{base}/run/{job.id}?workspace={job.workspace_id}">
								lock deps of {truncateHash(job.script_hash ?? '')}
							</a>
						{:else if 'job_kind' in job && job.job_kind == 'identity'}
							<a href="{base}/run/{job.id}?workspace={job.workspace_id}">no op</a>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		{#if job && job.parent_job}
			{#if job.is_flow_step}
				<div class="flex flex-row gap-1 items-center">
					<BarsStaggered class="text-secondary" size={14} />
					<span class="mx-1 text-xs">
						Step of flow <a href={`${base}/run/${job.parent_job}?workspace=${job.workspace_id}`}>
							{truncateRev(job.parent_job, 6)}
						</a>
					</span>
				</div>
			{:else}
				<div class="flex flex-row gap-1 items-center">
					<span class="text-2xs text-tertiary truncate">
						parent <a href={`${base}/run/${job.parent_job}?workspace=${job.workspace_id}`}>
							{truncateRev(job.parent_job, 10)}
						</a>
					</span>
				</div>
			{/if}
		{/if}
	</div>
	{#if containsLabel}
		<div class="w-3/12 flex justify-start px-0.5">
			{#if job && job?.['labels']}
				<div class="flex flex-row items-center gap-1 overflow-x-auto">
					{#if Array.isArray(job?.['labels'])}
						{#each job?.['labels'] as label}
							<Button
								variant="border"
								size="xs3"
								btnClasses={twMerge(
									activeLabel == label ? 'bg-blue-50 dark:bg-blue-900/50' : '',
									'!text-2xs !font-normal truncate max-w-28'
								)}
								color="light"
								on:click={() => {
									dispatch('filterByLabel', label)
								}}
								endIcon={{ icon: ListFilter }}
							>
								{label}
							</Button>
						{/each}
					{/if}
				</div>
			{/if}
		</div>
	{/if}
	<div class="w-3/12 flex justify-start">
		{#if job && job.schedule_path}
			<div class="flex flex-row items-center gap-1">
				<Calendar size={14} />
				<Button
					size="xs2"
					color="light"
					btnClasses="font-normal"
					on:click={() => scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
				>
					<div class="truncate text-ellipsis text-left" title={job.schedule_path}>
						{truncateRev(job.schedule_path, 20)}
					</div>
				</Button>
				<Button
					size="xs2"
					color="light"
					on:click={() => {
						dispatch('filterBySchedule', job.schedule_path)
					}}
				>
					<ListFilter size={10} />
				</Button>
			</div>
		{:else}
			<div class="flex flex-row gap-1 items-center">
				<div class="text-xs truncate text-ellipsis text-left" title={job.created_by}>
					{truncateRev(job.created_by ?? '', 20)}
				</div>
				{#if !isExternal}
					<Button
						size="xs2"
						color="light"
						on:click={() => {
							dispatch('filterByUser', job.created_by ?? '')
						}}
					>
						<ListFilter size={10} />
					</Button>
				{/if}
			</div>
		{/if}
	</div>
</div>
