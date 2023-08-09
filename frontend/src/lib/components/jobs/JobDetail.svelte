<script lang="ts">
	import { goto } from '$app/navigation'
	import type { Job } from '$lib/gen'
	import {
		displayDate,
		displayDaysAgo,
		forLater,
		msToSec,
		truncateHash,
		truncateRev
	} from '$lib/utils'
	import { faClock, faRobot, faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import { onDestroy, onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Badge, Button } from '../common'
	import ScheduleEditor from '../ScheduleEditor.svelte'
	import Row from '../table/Row.svelte'
	import Cell from '../table/Cell.svelte'

	const SMALL_ICON_SCALE = 0.7

	export let job: Job
	export let selectedId: string | undefined = undefined
	let scheduleEditor: ScheduleEditor

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

	function displayIfRecent(date: Date): string {
		const d = new Date(date)
		const now = new Date()
		const diff = now.getTime() - d.getTime()
		if (diff < 1000 * 600) {
			return `(${displayDaysAgo(d.toString())})`
		} else {
			return ''
		}
	}
	function endedDate(started_at: string, duration_ms: number): string {
		const started = new Date(started_at)
		started.setMilliseconds(started.getMilliseconds() + duration_ms)
		return `${displayDate(started)} ${displayIfRecent(started)}`
	}
</script>

<ScheduleEditor on:update={() => goto('/schedules')} bind:this={scheduleEditor} />

<Row
	hoverable
	selected={selectedId === job.id}
	on:click={() => {
		selectedId = job.id
	}}
>
	<Cell first>
		<div class="flex flex-row text-sm">
			{#if job === undefined}
				No job found
			{:else}
				<div class="flex flex-row space-x-2">
					<div class="whitespace-nowrap text-xs">
						{#if job.script_path}
							<a href="/run/{job.id}?workspace={job.workspace_id}">{job.script_path} </a>
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
	</Cell>
	<Cell>
		{job.created_by}
	</Cell>
	<Cell>
		<div class="flex flex-row gap-1">
			<Badge>
				{#if 'started_at' in job && job.started_at}
					<div>
						<span>
							{#if job?.['duration_ms']}
								Ended {#key time}
									{endedDate(job.started_at, job?.['duration_ms'])}{/key}
							{:else}
								Started {#key time}
									{displayDaysAgo(job.started_at ?? '')}{/key}
							{/if}</span
						>
					</div>
				{/if}
				{#if 'scheduled_for' in job && !job.running}
					Waiting for an executor
				{/if}
				<div>
					{#if job && job.parent_job}
						{#if job.is_flow_step}
							<Icon class="text-secondary" data={faBarsStaggered} scale={SMALL_ICON_SCALE} /><span
								class="mx-1"
							>
								Step of flow <a href={`/run/${job.parent_job}`}>{truncateRev(job.parent_job, 6)}</a
								></span
							>
						{:else}
							<Icon class="text-secondary" data={faRobot} scale={SMALL_ICON_SCALE} /><span
								class="mx-1"
							>
								Parent <a href={`/run/${job.parent_job}`}>{job.parent_job}</a></span
							>
						{/if}
					{/if}
				</div>
			</Badge>
			<div>
				{#if 'success' in job && job.success}
					{#if job.is_skipped}
						<Badge color="green">Skipped</Badge>
					{:else}
						<Badge color="green">
							Success
							{#if job && 'duration_ms' in job && job.duration_ms != undefined}
								({msToSec(job.duration_ms)}s)
							{/if}
						</Badge>
					{/if}
				{:else if 'success' in job}
					<Badge color="red">Error</Badge>
				{:else if 'running' in job && job.running}
					<Badge color="yellow">Job is running</Badge>
				{:else if job && 'running' in job && job.scheduled_for && forLater(job.scheduled_for)}
					<Badge color="blue">
						Scheduled

						{#if 'scheduled_for' in job && !job.running && job.scheduled_for && forLater(job.scheduled_for)}
							for {displayDate(job.scheduled_for ?? '')}
						{/if}
					</Badge>
				{:else}
					<Badge>Waiting for an executor</Badge>
				{/if}
			</div>
		</div>
	</Cell>
	<Cell last>
		<div class="flex flex-row gap-1">
			{#if job && job.schedule_path}
				<Button
					class="xs"
					color="light"
					on:click={() => scheduleEditor?.openEdit(job.schedule_path ?? '', job.job_kind == 'flow')}
				>
					Edit schedule
				</Button>
			{/if}
		</div>
	</Cell>
</Row>
