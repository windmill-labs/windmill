<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Job } from '$lib/gen'
	import { displayDate, msToSec, truncateHash, truncateRev } from '$lib/utils'
	import { faRobot, faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Badge, Button } from '../common'
	import ScheduleEditor from '../ScheduleEditor.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'
	import { Calendar, Check, FastForward, Hourglass, ListFilter, Play, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import TimeAgo from '../TimeAgo.svelte'
	import { forLater } from '$lib/forLater'

	const dispatch = createEventDispatcher()
	const SMALL_ICON_SCALE = 0.7

	export let job: Job
	export let selectedId: string | undefined = undefined
	let scheduleEditor: ScheduleEditor

	function endedDate(started_at: string, duration_ms: number): string {
		const started = new Date(started_at)
		started.setMilliseconds(started.getMilliseconds() + duration_ms)
		return `${started.toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		})}`
	}
</script>

<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />

<Row
	hoverable
	selected={selectedId === job.id}
	on:click={() => {
		selectedId = job.id
		dispatch('select')
	}}
>
	<Cell first>
		<div>
			{#if 'success' in job && job.success}
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
			{:else if 'running' in job && job.running}
				<Badge color="yellow" baseClass="!px-1.5">
					<Play size={14} />
				</Badge>
			{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
				<Badge color="blue" baseClass="!px-1.5">
					<Calendar size={14} />
				</Badge>
			{:else}
				<Badge baseClass="!px-1.5">
					<Hourglass size={14} />
				</Badge>
			{/if}
		</div>
	</Cell>
	<Cell>
		<div class="flex flex-row items-center gap-1 text-gray-500 dark:text-gray-300 text-2xs">
			{#if job}
				{#if 'started_at' in job && job.started_at}
					{#if job?.['duration_ms']}
						Ended {endedDate(job.started_at, job?.['duration_ms'])}
						{#if job && 'duration_ms' in job && job.duration_ms != undefined}
							(Ran in {msToSec(job.duration_ms)}s)
						{/if}
					{:else}
						<div>
							Started
							<TimeAgo date={job.started_at ?? ''} />
						</div>
					{/if}
				{:else if `scheduled_for` in job && job.scheduled_for && forLater(job.scheduled_for)}
					Scheduled for {displayDate(job.scheduled_for)}
				{:else}
					Waiting for executor (created <TimeAgo date={job.created_at || ''} />)
				{/if}
			{/if}
		</div>
	</Cell>

	<Cell>
		<div class="flex flex-row text-sm">
			{#if job === undefined}
				No job found
			{:else}
				<div class="flex flex-row space-x-2">
					<div class="whitespace-nowrap text-xs font-semibold">
						{#if job.script_path}
							<div class="flex flex-row gap-1 items-center">
								<a href="/run/{job.id}?workspace={job.workspace_id}">{job.script_path} </a>
								<Button
									size="xs2"
									color="light"
									on:click={() => {
										dispatch('filterByPath', job.script_path)
									}}
								>
									<ListFilter size={10} />
								</Button>
							</div>
						{:else if 'job_kind' in job && job.job_kind == 'preview'}
							<a href="/run/{job.id}?workspace={job.workspace_id}">Preview without path </a>
						{:else if 'job_kind' in job && job.job_kind == 'dependencies'}
							<a href="/run/{job.id}?workspace={job.workspace_id}"
								>lock deps of {truncateHash(job.script_hash ?? '')}</a
							>
						{:else if 'job_kind' in job && job.job_kind == 'identity'}
							<a href="/run/{job.id}?workspace={job.workspace_id}">no op</a>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		{#if job && job.parent_job}
			{#if job.is_flow_step}
				<Icon class="text-secondary" data={faBarsStaggered} scale={SMALL_ICON_SCALE} />
				<span class="mx-1">
					Step of flow <a href={`/run/${job.parent_job}`}>{truncateRev(job.parent_job, 6)} </a>
				</span>
			{:else}
				<Icon class="text-secondary" data={faRobot} scale={SMALL_ICON_SCALE} />
				<span class="mx-1">
					Parent <a href={`/run/${job.parent_job}`}>{job.parent_job}</a>
				</span>
			{/if}
		{/if}
	</Cell>
	<Cell last>
		{#if job && job.schedule_path}
			<div class="flex flex-row items-center gap-1">
				<Calendar size={14} />
				<Button
					size="xs2"
					color="light"
					btnClasses="font-normal"
					on:click={() => scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
				>
					{job.schedule_path}
				</Button>
			</div>
		{:else}
			{job.created_by}
		{/if}
	</Cell>
</Row>
