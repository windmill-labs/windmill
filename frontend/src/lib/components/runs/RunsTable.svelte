<script lang="ts">
	import type { Job } from '$lib/gen'
	import RunRow from './RunRow.svelte'
	import VirtualList from 'svelte-tiny-virtual-list'
	import { createEventDispatcher, onMount } from 'svelte'
	//import InfiniteLoading from 'svelte-infinite-loading'

	export let jobs: Job[] | undefined = undefined
	export let selectedId: string | undefined = undefined
	export let selectedWorkspace: string | undefined = undefined
	// const loadMoreQuantity: number = 100

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
				date.setMilliseconds(date.getMilliseconds())

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

	$: groupedJobs = jobs ? groupJobsByDay(jobs) : undefined

	type FlatJobs =
		| {
				type: 'date'
				date: string
		  }
		| {
				type: 'job'
				job: Job
		  }

	function flattenJobs(groupedJobs: Record<string, Job[]>): Array<FlatJobs> {
		const flatJobs: Array<FlatJobs> = []

		for (const [date, jobsByDay] of Object.entries(groupedJobs)) {
			flatJobs.push({ type: 'date', date })
			for (const job of jobsByDay) {
				flatJobs.push({ type: 'job', job })
			}
		}

		return flatJobs
	}

	$: flatJobs = groupedJobs ? flattenJobs(groupedJobs) : undefined

	let stickyIndices: number[] = []

	$: {
		stickyIndices = []
		let index = 0
		for (const entry of flatJobs ?? []) {
			if (entry.type === 'date') {
				stickyIndices.push(index)
			}
			index++
		}
	}

	let tableHeight: number = 0
	let header: number = 0
	let containerWidth: number = 0
	// const MAX_ITEMS = 1000

	/*
	function infiniteHandler({ detail: { loaded, error, complete } }) {
		try {
			nbOfJobs += loadMoreQuantity

			if (nbOfJobs >= MAX_ITEMS) {
				complete()
			} else {
				loaded()
			}
		} catch (e) {
			error()
		}
	}
	*/

	function computeHeight() {
		tableHeight = document.querySelector('#runs-table-wrapper')!.parentElement?.clientHeight ?? 0
	}
	onMount(() => {
		computeHeight()
	})
	const dispatch = createEventDispatcher()
</script>

<svelte:window on:resize={() => computeHeight()} />

<div
	class="divide-y min-w-[640px] h-full"
	id="runs-table-wrapper"
	bind:clientWidth={containerWidth}
>
	<div
		class="flex flex-row bg-surface-secondary sticky top-0 w-full p-2 pr-4"
		bind:clientHeight={header}
	>
		<div class="w-1/12 text-2xs"
			>{jobs?.length == 1000 ? '1000+' : jobs ? jobs.length.toString() : '...'} jobs</div
		>
		<div class="w-4/12 text-xs font-semibold">Timestamp</div>
		<div class="w-4/12 text-xs font-semibold">Path</div>
		<div class="w-3/12 text-xs font-semibold">Triggered by</div>
	</div>

	<VirtualList
		width="100%"
		height={tableHeight - header}
		itemCount={flatJobs?.length ?? 3}
		itemSize={42}
		{stickyIndices}
	>
		<div slot="item" let:index let:style {style} class="w-full">
			{#if flatJobs}
				{@const jobOrDate = flatJobs[index]}

				{#if jobOrDate}
					{#if jobOrDate?.type === 'date'}
						<div class="bg-surface-secondary py-2 border-b font-semibold text-xs pl-5">
							{jobOrDate.date}
						</div>
					{:else}
						<div class="flex flex-row items-center h-full w-full">
							<RunRow
								job={jobOrDate.job}
								{selectedId}
								on:select={() => {
									selectedWorkspace = jobOrDate.job.workspace_id
									selectedId = jobOrDate.job.id
									dispatch('select')
								}}
								on:filterByPath
								on:filterByUser
								on:filterByFolder
								{containerWidth}
							/>
						</div>
					{/if}
				{:else}
					{JSON.stringify(jobOrDate)}
				{/if}
			{:else}
				<div class="flex flex-row items-center h-full w-full">
					<div class="w-1/12 text-2xs">...</div>
					<div class="w-4/12 text-xs">...</div>
					<div class="w-4/12 text-xs">...</div>
					<div class="w-3/12 text-xs">...</div>
				</div>
			{/if}
		</div>
	</VirtualList>
</div>
{#if jobs?.length == 0}
	<tr>
		<td colspan="4" class="text-center py-8">
			<div class="text-xs text-secondary"> No jobs found for the selected filters. </div>
		</td>
	</tr>
{/if}
