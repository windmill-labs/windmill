<script lang="ts">
	import type { Job } from '$lib/gen'
	import { ArrowDownIcon } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import RunRow from './RunRow.svelte'
	import VirtualList from 'svelte-tiny-virtual-list'

	export let jobs: Job[] = []
	export let selectedId: string | undefined = undefined
	export let nbOfJobs: number = 30
	const loadMoreQuantity: number = 100

	function getTime(job: Job): string | undefined {
		return job['started_at'] ?? job['scheduled_for'] ?? job['created_at']
	}
	function groupJobsByDay(jobs: Job[]): Record<string, Job[]> {
		const groupedLogs: Record<string, Job[]> = {}

		if (!jobs) return groupedLogs

		for (const job of jobs) {
			const field: string | undefined = getTime(job)
			if (field) {
				const date = new Date(field)
				date.setMilliseconds(date.getMilliseconds() + (job['duration_ms'] ?? 0))

				const day = date.toLocaleDateString('en-US', {
					year: 'numeric',
					month: 'long',
					day: 'numeric'
				})

				if (!groupedLogs[day]) {
					groupedLogs[day] = []
				}

				groupedLogs[day].push(job)
			}
		}

		for (const day in groupedLogs) {
			groupedLogs[day].sort((a, b) => {
				return new Date(getTime(b)!).getTime() - new Date(getTime(a)!).getTime()
			})
		}

		const sortedLogs: Record<string, Job[]> = {}
		Object.keys(groupedLogs)
			.sort((a, b) => {
				return new Date(b).getTime() - new Date(a).getTime()
			})
			.forEach((key) => {
				sortedLogs[key] = groupedLogs[key]
			})

		return sortedLogs
	}

	$: groupedJobs = groupJobsByDay(jobs.slice(0, nbOfJobs))

	let flatJobs: Array<
		| {
				type: 'date'
				date: string
		  }
		| {
				type: 'job'
				job: Job
		  }
	> = []
	$: {
		flatJobs = []
		for (const [date, jobsByDay] of Object.entries(groupedJobs)) {
			flatJobs.push({ type: 'date', date })
			for (const job of jobsByDay) {
				flatJobs.push({ type: 'job', job })
			}
		}
	}

	let stickyIndices: number[] = []

	$: {
		stickyIndices = []
		let index = 0
		for (const entry of flatJobs) {
			if (entry.type === 'date') {
				stickyIndices.push(index)
			}
			index++
		}
	}
</script>

<div class="divide-y h-full">
	<div class="flex flex-row bg-surface-secondary sticky top-0">
		<div class="w-8" />
		<div>Timestamp</div>
		<div>Path</div>
		<div>Triggered by</div>
	</div>

	<VirtualList width="100%" height="100%" itemCount={flatJobs.length} itemSize={50}>
		<div slot="item" let:index let:style {style}>
			{@const jobOrDate = flatJobs[index]}
			{#if jobOrDate.type === 'date'}
				<div class="bg-surface-secondary/30 py-2 border-b font-semibold">
					{jobOrDate.date}
				</div>
			{:else}
				<RunRow
					job={jobOrDate.job}
					bind:selectedId
					on:select
					on:filterByPath
					on:filterByUser
					on:filterByFolder
				/>
			{/if}
		</div>
	</VirtualList>
</div>
{#if jobs.length == 0}
	<tr>
		<td colspan="4" class="text-center py-8">
			<div class="text-xs text-secondary"> No jobs found for the selected filters. </div>
		</td>
	</tr>
{/if}
{#if nbOfJobs < jobs.length}
	<div class="bg-surface border-t flex flex-row justify-center py-4 items-center gap-2">
		<Button color="light" size="xs2" on:click={() => (nbOfJobs += loadMoreQuantity)}>
			<div class="flex flex-row gap-1 items-center">
				Load {loadMoreQuantity} more
				<ArrowDownIcon size={16} />
			</div>
		</Button>
	</div>
{/if}
