<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import type { Job } from '$lib/gen'
	import {
		displayDate,
		truncateHash,
		truncateRev,
		isScriptPreview,
		msToReadableTime,
		isFlowPreview,
		getJobKindIcon
	} from '$lib/utils'
	import { Button } from '../common'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import JobStatusIcon from '$lib/components/runs/JobStatusIcon.svelte'

	import { Calendar, Clock, ExternalLink, ListFilterPlus } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import TimeAgo from '../TimeAgo.svelte'
	import { forLater } from '$lib/forLater'
	import { twMerge } from 'tailwind-merge'
	import Portal from '$lib/components/Portal.svelte'

	import WaitTimeWarning from '../common/waitTimeWarning/WaitTimeWarning.svelte'
	import DropdownV2 from '../DropdownV2.svelte'
	import { Tooltip } from '../meltComponents'
	import RunLabels from './RunLabels.svelte'
	import './runs-grid.css'

	const dispatch = createEventDispatcher()

	interface Props {
		job: Job
		selected?: boolean
		containerWidth?: number
		containsLabel?: boolean
		showTag?: boolean
		activeLabel: string | null
		manualSelectionMode?: undefined | 'cancel' | 'rerun'
	}

	let {
		job,
		selected = false,
		containerWidth = 0,
		containsLabel = false,
		showTag = true,
		activeLabel,
		manualSelectionMode
	}: Props = $props()

	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	let isExternal = $derived(job && job.id === '-')

	let labelWidth = $state(0)

	let isJobRecent = $state(true)
</script>

<Portal name="run-row">
	<ScheduleEditor onUpdate={() => goto('/schedules')} bind:this={scheduleEditor} />
</Portal>
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'cursor-pointer',
		selected ? 'bg-surface-accent-selected' : 'hover:bg-surface-hover',
		'grid items-center h-full'
	)}
	class:grid-runs-table={!containsLabel && !manualSelectionMode && showTag}
	class:grid-runs-table-with-labels={containsLabel && !manualSelectionMode && showTag}
	class:grid-runs-table-selection={!containsLabel && manualSelectionMode && showTag}
	class:grid-runs-table-with-labels-selection={containsLabel && manualSelectionMode && showTag}
	class:grid-runs-table-no-tag={!containsLabel && !manualSelectionMode && !showTag}
	class:grid-runs-table-with-labels-no-tag={containsLabel && !manualSelectionMode && !showTag}
	class:grid-runs-table-selection-no-tag={!containsLabel && manualSelectionMode && !showTag}
	class:grid-runs-table-with-labels-selection-no-tag={containsLabel &&
		manualSelectionMode &&
		!showTag}
	style="width: {containerWidth}px"
	onclick={() => dispatch('select')}
	oncontextmenu={(e) => !selected && dispatch('select')}
>
	<!-- Selection column (only when in selection mode) -->
	{#if manualSelectionMode}
		<div class="w-4 h-4 ml-4 pointer-events-none">
			<input type="checkbox" checked={selected} />
		</div>
	{/if}

	<!-- Status -->
	<div class="flex items-center justify-start pl-4">
		<JobStatusIcon {job} {isExternal} />
	</div>

	<!-- Job time -->
	<div class="overflow-hidden min-w-0">
		<div class="flex flex-row items-center gap-1 text-secondary text-2xs">
			{#if job}
				{#if ('started_at' in job && job.started_at) || ('completed_at' in job && job.completed_at)}
					{#if 'completed_at' in job && job.completed_at}
						{isJobRecent ? 'Ended' : ''}
						<TimeAgo bind:isRecent={isJobRecent} agoOnlyIfRecent date={job.completed_at ?? ''} />
					{:else if 'started_at' in job && job.started_at}
						{isJobRecent ? 'Started' : ''}
						<TimeAgo bind:isRecent={isJobRecent} agoOnlyIfRecent date={job.started_at ?? ''} />
					{/if}
					{#if job && (job.self_wait_time_ms || job.aggregate_wait_time_ms)}
						<WaitTimeWarning
							self_wait_time_ms={job.self_wait_time_ms}
							aggregate_wait_time_ms={job.aggregate_wait_time_ms}
							variant="icon"
						/>
					{/if}
				{:else if `scheduled_for` in job && job.scheduled_for && forLater(job.scheduled_for)}
					{displayDate(job.scheduled_for)}<Clock size={12} />
				{:else if job.canceled}
					{#if job.type == 'CompletedJob'}
						Cancelled <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />
					{:else}
						Cancelling job... (created <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />)
					{/if}
				{:else if `scheduled_for` in job && job.scheduled_for && forLater(job.scheduled_for)}
					Waiting executor (<TimeAgo agoOnlyIfRecent date={job.scheduled_for || ''} />)
				{:else}
					Waiting executor (<TimeAgo agoOnlyIfRecent date={job.created_at || ''} />)
				{/if}
			{/if}
		</div>
	</div>

	<!-- Job duration-->
	<div class="text-2xs font-normal text-primary pr-2">
		{#if job && 'duration_ms' in job && job.duration_ms != undefined}
			{msToReadableTime(job.duration_ms, 2)}
		{:else}
			-
		{/if}
	</div>

	<!-- Job path-->
	<div class="flex justify-start flex-col pr-4">
		{#if job === undefined}
			No job found
		{:else}
			{@const JobKindIcon = getJobKindIcon(job.job_kind)}
			<div class="flex flex-row gap-3 min-w-0 items-center h-full">
				<Tooltip class="h-full">
					<div class="relative">
						{#if job && job.parent_job}
							<span class="absolute -top-1 -right-1 text-xs text-accent">*</span>
						{/if}
						<JobKindIcon size={14} />
					</div>
					{#snippet text()}
						<span>
							{#if job && job.job_kind}
								{job.job_kind}
							{/if}
							{#if job && job.is_flow_step && job.parent_job}
								<br /> Step of flow
								<a href={`${base}/run/${job.parent_job}?workspace=${job.workspace_id}`}>
									{truncateRev(job.parent_job, 10)}
								</a>
							{:else if job && job.parent_job}
								<br /> Parent
								<a href={`${base}/run/${job.parent_job}?workspace=${job.workspace_id}`}>
									{truncateRev(job.parent_job, 10)}
								</a>
							{/if}
						</span>
					{/snippet}
				</Tooltip>

				<div class="whitespace-nowrap text-xs text-primary truncate">
					{#if job.script_path}
						<div class="flex flex-row gap-1 items-center">
							{#if isExternal}
								<span class="w-30 justify-center">-</span>
							{:else}
								<span class="truncate w-30" title={job.script_path}>
									{job.script_path}
								</span>
							{/if}
							{#if !isExternal || job.script_path?.startsWith('f/')}
								{@const isFolder = job.script_path?.startsWith('f/')}
								<DropdownV2
									items={() => {
										const items = isExternal
											? []
											: [
													{
														displayName: `Filter by path: ${job.script_path}`,
														action: () => dispatch('filterByPath', job.script_path),
														disabled: isExternal
													}
												]
										if (isFolder) {
											const folder = job.script_path?.split('/')[1]
											return [
												{
													displayName: `Filter by folder: ${folder}`,
													action: () => dispatch('filterByFolder', folder)
												},
												...items
											]
										}
										return items
									}}
									class="w-fit"
								>
									{#snippet buttonReplacement()}
										{@render filterButton()}
									{/snippet}
								</DropdownV2>
							{/if}
						</div>
					{:else if 'job_kind' in job && isScriptPreview(job.job_kind)}
						Preview without path
					{:else if 'job_kind' in job && job.job_kind == 'dependencies'}
						lock deps of {truncateHash(job.script_hash ?? '')}
					{:else if 'job_kind' in job && job.job_kind == 'identity'}
						no op
					{:else if 'job_kind' in job && isFlowPreview(job.job_kind)}
						Preview without path
					{/if}
				</div>
			</div>
		{/if}
	</div>
	<!-- Labels-->
	{#if containsLabel}
		<div class="flex justify-start overflow-hidden" bind:clientWidth={labelWidth}>
			<RunLabels
				{job}
				{activeLabel}
				onFilterByLabel={(label) => dispatch('filterByLabel', label)}
				{labelWidth}
			/>
		</div>
	{/if}
	<!-- Author and schedule-->
	<div class="flex justify-start pr-4 text-primary">
		{#if job && job.schedule_path}
			<div class="flex flex-row items-center gap-1 w-full -ml-2">
				<Button
					size="xs2"
					color="light"
					btnClasses="font-normal bg-transparent hover:bg-surface hover:text-primary"
					on:click={() => scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
				>
					<Calendar size={14} />
				</Button>
				<div
					class="text-xs text-primary font-normal truncate text-ellipsis text-left"
					dir="rtl"
					title={job.schedule_path}
				>
					{job.schedule_path}
				</div>
				<DropdownV2
					items={[
						{
							displayName: `Filter by schedule: ${truncateRev(job.schedule_path, 20)}`,
							action: () => dispatch('filterBySchedule', job.schedule_path)
						}
					]}
					class="w-fit"
				>
					{#snippet buttonReplacement()}
						{@render filterButton()}
					{/snippet}
				</DropdownV2>
			</div>
		{:else}
			<div class="flex flex-row gap-1 items-center w-full">
				<div
					class="text-xs font-normal text-primary truncate text-ellipsis text-left"
					dir="rtl"
					title={job.created_by}
				>
					{job.created_by ?? ''}
				</div>
				{#if !isExternal}
					<DropdownV2
						items={[
							{
								displayName: `Filter by triggered by: ${job.created_by}`,
								action: () => dispatch('filterByUser', job.created_by ?? '')
							}
						]}
						customWidth={256}
						class="w-fit"
					>
						{#snippet buttonReplacement()}
							{@render filterButton()}
						{/snippet}
					</DropdownV2>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Job tag-->
	{#if showTag}
		<div class="flex justify-start gap-1">
			{#if job.tag}
				<span class="text-xs text-primary font-normal truncate" title={job.tag}>{job.tag}</span>
			{/if}
		</div>
	{/if}

	<!-- Job link-->
	{#if !isExternal}
		<div class="flex justify-end pr-2">
			<a
				target="_blank"
				href="{base}/run/{job.id}?workspace={job.workspace_id}"
				class={twMerge(
					'text-right float-right  px-2',
					selected ? 'text-accent' : 'text-secondary hover:text-accent'
				)}
				title="See run detail in a new tab"
			>
				<ExternalLink size={14} />
			</a>
		</div>
	{/if}
</div>

{#snippet filterButton()}
	<div class="p-1 cursor-pointer rounded-md text-hint/50 hover:text-primary">
		<ListFilterPlus size={14} />
	</div>
{/snippet}
