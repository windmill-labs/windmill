<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { Job } from '$lib/gen'
	import RunRow from './RunRow.svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { createEventDispatcher, onMount } from 'svelte'
	import Tooltip from '../Tooltip.svelte'
	import { AlertTriangle } from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { workspaceStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { isJobSelectable } from '$lib/utils'
	import type { RunsSelectionMode } from './RunsBatchActionsDropdown.svelte'

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
		lastFetchWentToEnd = false
	}: Props = $props()

	function getTime(job: Job): string | undefined {
		return job['started_at'] ?? job['scheduled_for'] ?? job['created_at']
	}

	let containsLabel = $state(false)
	function groupJobsByDay(jobs: Job[]): Record<string, Job[]> {
		const groupedLogs: Record<string, Job[]> = {}

		if (!jobs) return groupedLogs

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

				if (!groupedLogs[day]) {
					groupedLogs[day] = []
				}

				groupedLogs[day].push(job)
			}
		}
		containsLabel = newContainsLabel

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

	let selectableJobCount = $derived.by(() => {
		if (!selectionMode) return 0
		return jobs?.filter(isJobSelectable(selectionMode)).length ?? 0
	})
	let allSelected = $derived.by(() => {
		return selectionMode && selectedIds.length === selectableJobCount
	})

	function selectAll() {
		if (!selectionMode) return
		if (allSelected) {
			allSelected = false
			selectedIds = []
		} else {
			allSelected = true
			selectedIds = jobs?.filter(isJobSelectable(selectionMode)).map((j) => j.id) ?? []
		}
	}

	function jobCountString(jobCount: number | undefined, lastFetchWentToEnd: boolean): string {
		if (jobCount === undefined) {
			return ''
		}
		const jc = jobCount
		const isTruncated = jc >= 1000 && !lastFetchWentToEnd

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
	let groupedJobs = $derived(
		jobs
			? showExternalJobs
				? groupJobsByDay([...jobs, ...externalJobs])
				: groupJobsByDay(jobs)
			: undefined
	)
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
</script>

<svelte:window onresize={() => computeHeight()} />

<div
	class="divide-y min-w-[640px] h-full"
	id="runs-table-wrapper"
	bind:clientWidth={containerWidth}
>
	<div bind:clientHeight={headerHeight}>
		{#if selectionMode && selectableJobCount}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={twMerge(
					'hover:bg-surface-hover bg-surface-primary cursor-pointer',
					allSelected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
					'flex flex-row items-center sticky w-full p-2 pr-4 top-0 font-semibold border-t text-sm'
				)}
				onclick={selectAll}
			>
				<div class="px-2">
					<input onfocus={bubble('focus')} type="checkbox" checked={allSelected} />
				</div>
				Select all
			</div>
		{/if}
		<div class="flex flex-row bg-surface-secondary sticky top-0 w-full p-2 pr-4">
			{#if showExternalJobs && externalJobs.length > 0}
				<div class="w-1/12 text-2xs">
					<div class="flex flex-row">
						{jobs
							? jobCountString(jobs.length + externalJobs.length, lastFetchWentToEnd)
							: ''}<Tooltip>{externalJobs.length} jobs obscured</Tooltip>
					</div>
				</div>
			{:else if $workspaceStore !== 'admins' && omittedObscuredJobs}
				<div class="w-1/12 text-2xs flex flex-row">
					{jobs ? jobCountString(jobs.length, lastFetchWentToEnd) : ''}
					<Popover>
						<AlertTriangle size={16} class="ml-0.5 text-yellow-500" />
						{#snippet text()}
							Too specific filtering may have caused the omission of obscured jobs. This is done for
							security reasons. To see obscured jobs, try removing some filters.
						{/snippet}
					</Popover>
				</div>
			{:else}
				<div class="w-1/12 text-2xs"
					>{jobs ? jobCountString(jobs.length, lastFetchWentToEnd) : ''}</div
				>
			{/if}
			<div class="w-4/12 text-xs font-semibold"></div>
			<div class="w-4/12 text-xs font-semibold">Path</div>
			{#if containsLabel}
				<div class="w-3/12 text-xs font-semibold">Label</div>
			{/if}
			<div class="w-3/12 text-xs font-semibold">Triggered by</div>
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
			scrollToBehaviour="smooth"
		>
			{#snippet header()}{/snippet}
			{#snippet children({ index, style })}
				<div {style} class="w-full">
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
										{containsLabel}
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
												selectedWorkspace = jobOrDate.job.workspace_id
												selectedIds = [jobOrDate.job.id]
												dispatch('select')
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
					>{#if !lastFetchWentToEnd && jobs && jobs.length >= 1000}
						<button
							class="text-xs text-blue-600 text-center w-full pb-2"
							onclick={() => {
								dispatch('loadExtra')
							}}>Load next 1000 jobs</button
						>
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
