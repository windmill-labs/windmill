<script lang="ts">
	import type { Job } from '$lib/gen'
	import RunRow from './RunRow.svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { createEventDispatcher, onMount } from 'svelte'
	import Tooltip from '../Tooltip.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { workspaceStore } from '$lib/stores'
	import './runs-grid.css'
	import type { RunsSelectionMode } from '$lib/utils'

	interface Props {
		//import InfiniteLoading from 'svelte-infinite-loading'
		jobs?: Job[] | undefined
		externalJobs?: Job[]
		omittedObscuredJobs: boolean
		showExternalJobs?: boolean
		selectionMode?: RunsSelectionMode | false
		selectedIds?: string[]
		selectedWorkspace?: string | undefined
		activeLabel?: string | null
		// const loadMoreQuantity: number = 100
		lastFetchWentToEnd?: boolean
		perPage?: number
	}

	let {
		jobs = undefined,
		externalJobs = [],
		omittedObscuredJobs,
		showExternalJobs = false,
		selectionMode = false,
		selectedIds = $bindable([]),
		selectedWorkspace = $bindable(undefined),
		activeLabel = null,
		lastFetchWentToEnd = false,
		perPage = 1000
	}: Props = $props()

	function getTime(job: Job): string | undefined {
		return job['completed_at'] ?? job['started_at'] ?? job['scheduled_for'] ?? job['created_at']
	}

	function groupJobsByDay(jobs: Job[]): {
		groupedJobs: Record<string, Job[]>
		newContainsLabel: boolean | undefined
	} {
		const groupedJobs: Record<string, Job[]> = {}

		if (!jobs) return { groupedJobs, newContainsLabel: undefined }

		let newContainsLabel = false
		for (const job of jobs) {
			if (job?.['labels'] != undefined) {
				newContainsLabel = true
			}
			const field: string | undefined = getTime(job)
			if (field) {
				const date = new Date(field)
				date.setMilliseconds(date.getMilliseconds())

				const day = date.toLocaleDateString('en-US', {
					year: 'numeric',
					month: 'long',
					day: 'numeric'
				})

				if (!groupedJobs[day]) {
					groupedJobs[day] = []
				}

				groupedJobs[day].push(job)
			}
		}

		for (const day in groupedJobs) {
			groupedJobs[day].sort((a, b) => {
				return new Date(getTime(b)!).getTime() - new Date(getTime(a)!).getTime()
			})
		}

		const sortedJobs: Record<string, Job[]> = {}
		Object.keys(groupedJobs)
			.sort((a, b) => {
				return new Date(b).getTime() - new Date(a).getTime()
			})
			.forEach((key) => {
				sortedJobs[key] = groupedJobs[key]
			})

		return { groupedJobs: sortedJobs, newContainsLabel }
	}

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

	let tableHeight: number = $state(0)
	let headerHeight: number = $state(0)
	let containerWidth: number = $state(0)
	// const MAX_ITEMS = perPage

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

	function jobCountString(jobCount: number | undefined, lastFetchWentToEnd: boolean): string {
		if (jobCount === undefined) {
			return ''
		}
		const jc = jobCount
		const isTruncated = jc >= perPage && !lastFetchWentToEnd

		return `${jc}${isTruncated ? '+' : ''} job${jc != 1 ? 's' : ''}`
	}

	function computeHeight() {
		tableHeight = document.querySelector('#runs-table-wrapper')!.parentElement?.clientHeight ?? 0
	}
	onMount(() => {
		computeHeight()
	})
	const dispatch = createEventDispatcher()

	let scrollToIndex = $state(0)

	export function scrollToRun(ids: string[]) {
		if (flatJobs && ids.length > 0) {
			const i = flatJobs.findIndex(
				(jobOrDate) => jobOrDate.type === 'job' && jobOrDate.job.id === ids[0]
			)
			if (i !== -1) {
				scrollToIndex = i
			}
		}
	}
	let { groupedJobs, newContainsLabel } = $derived(
		jobs
			? showExternalJobs
				? groupJobsByDay([...jobs, ...externalJobs])
				: groupJobsByDay(jobs)
			: { groupedJobs: undefined, newContainsLabel: undefined }
	)

	let containsLabel = $state(false)
	$effect(() => {
		if (newContainsLabel != undefined) {
			containsLabel = newContainsLabel
		}
	})
	let flatJobs = $derived(groupedJobs ? flattenJobs(groupedJobs) : undefined)
	let stickyIndices = $derived.by(() => {
		const nstickyIndices: number[] = []
		let index = 0
		for (const entry of flatJobs ?? []) {
			if (entry.type === 'date') {
				nstickyIndices.push(index)
			}
			index++
		}
		return nstickyIndices
	})

	const showTag = $derived(containerWidth > 700)
</script>

<svelte:window onresize={() => computeHeight()} />

<div
	class="divide-y h-full border min-w-[650px]"
	id="runs-table-wrapper"
	bind:clientWidth={containerWidth}
>
	<div bind:clientHeight={headerHeight}>
		<div
			class="grid bg-surface-secondary sticky top-0 w-full py-2 pr-4"
			class:grid-runs-table={!containsLabel && !selectionMode && showTag}
			class:grid-runs-table-with-labels={containsLabel && !selectionMode && showTag}
			class:grid-runs-table-selection={!containsLabel && selectionMode && showTag}
			class:grid-runs-table-with-labels-selection={containsLabel && selectionMode && showTag}
			class:grid-runs-table-no-tag={!containsLabel && !selectionMode && !showTag}
			class:grid-runs-table-with-labels-no-tag={containsLabel && !selectionMode && !showTag}
			class:grid-runs-table-selection-no-tag={!containsLabel && selectionMode && !showTag}
			class:grid-runs-table-with-labels-selection-no-tag={containsLabel &&
				selectionMode &&
				!showTag}
		>
			{#if selectionMode}
				<div class="text-xs font-semibold pl-4"></div>
			{/if}
			<div class="text-2xs px-2 flex flex-row items-center gap-2">
				{#if showExternalJobs && externalJobs.length > 0}
					<div class="flex flex-row">
						{jobs
							? jobCountString(jobs.length + externalJobs.length, lastFetchWentToEnd)
							: ''}<Tooltip>{externalJobs.length} jobs obscured</Tooltip>
					</div>
				{:else if $workspaceStore !== 'admins' && omittedObscuredJobs}
					<div class="flex flex-row">
						{jobs ? jobCountString(jobs.length, lastFetchWentToEnd) : ''}
						<Popover>
							<AlertTriangle size={16} class="ml-0.5 text-yellow-500" />
							{#snippet text()}
								Too specific filtering may have caused the omission of obscured jobs. This is done
								for security reasons. To see obscured jobs, try removing some filters.
							{/snippet}
						</Popover>
					</div>
				{:else}
					{jobs ? jobCountString(jobs.length, lastFetchWentToEnd) : ''}
				{/if}
			</div>
			<div class="text-xs font-semibold"></div>
			<div class="text-xs font-semibold">Duration</div>
			<div class="text-xs font-semibold">Path</div>
			{#if containsLabel}
				<div class="text-xs font-semibold">Label</div>
			{/if}
			<div class="text-xs font-semibold">Triggered by</div>
			{#if showTag}
				<div class="text-xs font-semibold">Tag</div>
			{/if}
			<div class=""></div>
		</div>
	</div>
	{#if jobs?.length == 0 && (!showExternalJobs || externalJobs?.length == 0)}
		<div class="text-xs text-secondary p-8"> No jobs found for the selected filters. </div>
	{:else}
		<VirtualList
			width="100%"
			height={tableHeight - headerHeight}
			itemCount={flatJobs?.length ?? 3}
			itemSize={42}
			overscanCount={20}
			{stickyIndices}
			{scrollToIndex}
			scrollToAlignment="center"
		>
			{#snippet header()}{/snippet}
			{#snippet item({ index, style })}
				<div {style} class="w-full">
					{#if flatJobs}
						{@const jobOrDate = flatJobs[index]}

						{#if jobOrDate}
							{#if jobOrDate?.type === 'date'}
								<div
									class="bg-surface-secondary py-2 font-semibold text-xs pl-2 h-[42px] flex items-center"
								>
									{jobOrDate.date}
								</div>
							{:else}
								<div class="flex flex-row items-center h-full w-full">
									<RunRow
										{containsLabel}
										{showTag}
										job={jobOrDate.job}
										selected={jobOrDate.job.id !== '-' && selectedIds.includes(jobOrDate.job.id)}
										{selectionMode}
										on:select={() => {
											const jobId = jobOrDate.job.id
											if (selectionMode) {
												if (selectedIds.includes(jobOrDate.job.id)) {
													selectedIds = selectedIds.filter((id) => id != jobId)
												} else {
													selectedIds.push(jobId)
													selectedIds = selectedIds
												}
											} else {
												if (
													JSON.stringify(selectedIds) !== JSON.stringify([jobOrDate.job.id]) ||
													selectedWorkspace !== jobOrDate.job.workspace_id
												) {
													selectedWorkspace = jobOrDate.job.workspace_id
													selectedIds = [jobOrDate.job.id]
													dispatch('select')
												} else {
													selectedIds = []
													selectedWorkspace = undefined
													dispatch('select')
												}
											}
										}}
										{activeLabel}
										on:filterByLabel
										on:filterByPath
										on:filterByUser
										on:filterByFolder
										on:filterByConcurrencyKey
										on:filterBySchedule
										on:filterByWorker
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
			{/snippet}
			{#snippet footer()}
				<div
					>{#if !lastFetchWentToEnd && jobs && jobs.length >= perPage}
						<button
							class="text-xs text-accent text-center w-full pb-2"
							onclick={() => {
								dispatch('loadExtra')
							}}
						>
							Load next {perPage} jobs
						</button>
					{/if}</div
				>
			{/snippet}
		</VirtualList>
	{/if}
</div>

<style>
	:global(.virtual-list-wrapper:hover::-webkit-scrollbar) {
		width: 8px !important;
		height: 8px !important;
	}
</style>
