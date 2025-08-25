<script lang="ts">
	import { base } from '$lib/base'
	import { goto } from '$lib/navigation'
	import type { Job } from '$lib/gen'
	import {
		displayDate,
		truncateHash,
		truncateRev,
		isScriptPreview,
		isJobSelectable,
		msToReadableTime
	} from '$lib/utils'
	import { Badge, Button } from '../common'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import {
		Calendar,
		Check,
		ExternalLink,
		FastForward,
		Hourglass,
		ListFilterPlus,
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
	import type { RunsSelectionMode } from './RunsBatchActionsDropdown.svelte'
	import RunBadges from './RunBadges.svelte'
	import DropdownV2 from '../DropdownV2.svelte'

	const dispatch = createEventDispatcher()

	interface Props {
		job: Job
		selected?: boolean
		containerWidth?: number
		containsLabel?: boolean
		activeLabel: string | null
		selectionMode?: RunsSelectionMode | false
	}

	let {
		job,
		selected = false,
		containerWidth = 0,
		containsLabel = false,
		activeLabel,
		selectionMode = false
	}: Props = $props()

	let scheduleEditor: ScheduleEditor | undefined = $state(undefined)

	let isExternal = $derived(job && job.id === '-')
</script>

<Portal name="run-row">
	<ScheduleEditor onUpdate={() => goto('/schedules')} bind:this={scheduleEditor} />
</Portal>
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={twMerge(
		'hover:bg-surface-hover cursor-pointer',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'flex flex-row items-center h-full'
	)}
	style="width: {containerWidth}px"
	onclick={() => {
		if (!selectionMode || isJobSelectable(selectionMode)(job)) {
			dispatch('select')
		}
	}}
>
	<!-- Flow status-->
	<div class="w-[5%] flex justify-center">
		{#if selectionMode && isJobSelectable(selectionMode)(job)}
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

	<!-- Job time-->
	<div class="w-[20%] flex justify-start pr-4">
		<div class="flex flex-row items-center gap-1 text-secondary text-2xs">
			{#if job}
				{#if 'started_at' in job && job.started_at}
					Started <TimeAgo agoOnlyIfRecent date={job.started_at ?? ''} />
					{#if job && 'duration_ms' in job && job.duration_ms != undefined}
						(Ran in {msToReadableTime(job.duration_ms)})
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
					{#if job.type == 'CompletedJob'}
						Cancelled <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />
					{:else}
						Cancelling job... (created <TimeAgo agoOnlyIfRecent date={job.created_at || ''} />)
					{/if}
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

	<!-- Job path-->
	<div class="w-[35%] flex justify-start flex-col pr-4">
		<div class="flex flex-row text-sm grow min-h-2">
			{#if job === undefined}
				No job found
			{:else}
				<div class="flex flex-row gap-1 min-w-0">
					<div class="whitespace-nowrap text-xs text-secondary truncate">
						{#if job.script_path}
							<div class="flex flex-row gap-1 items-center">
								{#if isExternal}
									<span class="w-30 justify-center">-</span>
								{:else}
									<span class="truncate w-30">
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
											<div
												class="p-1 hover:bg-surface cursor-pointer rounded-md text-gray-300 hover:text-primary"
											>
												<ListFilterPlus size={14} />
											</div>
										{/snippet}
									</DropdownV2>
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
				<div class="flex flex-row gap-1 items-center -mt-2 text-tertiary">
					<BarsStaggered size={10} />
					<span class="mx-1 text-2xs">
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
								endIcon={{ icon: ListFilterPlus }}
							>
								{label}
							</Button>
						{/each}
					{/if}
				</div>
			{/if}
		</div>
	{/if}
	<!-- Author and schedule-->
	<div class="w-[20%] flex justify-start pr-4 text-secondary">
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
				<div class="text-xs truncate text-ellipsis text-lef" dir="rtl" title={job.schedule_path}>
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
						<div
							class="p-1 hover:bg-surface cursor-pointer rounded-md text-gray-300 hover:text-primary"
						>
							<ListFilterPlus size={14} />
						</div>
					{/snippet}
				</DropdownV2>
			</div>
		{:else}
			<div class="flex flex-row gap-1 items-center w-full">
				<div class="text-xs truncate text-ellipsis text-left" dir="rtl" title={job.created_by}>
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
							<div
								class="p-1 hover:bg-surface cursor-pointer rounded-md text-gray-300 hover:text-primary"
							>
								<ListFilterPlus size={14} />
							</div>
						{/snippet}
					</DropdownV2>
				{/if}
			</div>
		{/if}
	</div>

	<div class="w-[15%] flex justify-start gap-1">
		<RunBadges {job} showScriptHash={false} verySmall />
	</div>

	<!-- Job link-->
	{#if !isExternal}
		<div class="w-[5%] flex justify-end">
			<a
				target="_blank"
				href="{base}/run/{job.id}?workspace={job.workspace_id}"
				class="text-right float-right text-gray-300 hover:text-primary dark:text-gray-300 px-2"
				title="See run detail in a new tab"
			>
				<ExternalLink size={14} />
			</a>
		</div>
	{/if}
</div>
