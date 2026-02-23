<script lang="ts">
	import type { Job } from '$lib/gen'
	import RunRow from './RunRow.svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { createEventDispatcher } from 'svelte'
	import Tooltip from '../Tooltip.svelte'
	import {
		AlertTriangle,
		CircleXIcon,
		Code2Icon,
		ExternalLinkIcon,
		RefreshCwIcon
	} from 'lucide-svelte'
	import Popover from '../Popover.svelte'
	import { workspaceStore } from '$lib/stores'
	import './runs-grid.css'
	import { useKeyPressed } from '$lib/svelte5Utils.svelte'
	import { twMerge } from 'tailwind-merge'
	import RightClickPopover from '../RightClickPopover.svelte'
	import DropdownMenu, { type Props as DropdownMenuProps } from '../DropdownMenu.svelte'
	import { clickOutside, isJobCancelable, isJobReRunnable } from '$lib/utils'
	import { goto } from '$lib/navigation'
	import BarsStaggered from '../icons/BarsStaggered.svelte'

	interface Props {
		//import InfiniteLoading from 'svelte-infinite-loading'
		jobs?: Job[] | undefined
		externalJobs?: Job[]
		omittedObscuredJobs: boolean
		showExternalJobs?: boolean
		selectedIds?: string[]
		selectedWorkspace?: string | undefined
		activeLabel?: string | null
		// const loadMoreQuantity: number = 100
		lastFetchWentToEnd?: boolean
		perPage?: number
		batchRerunOptionsIsOpen?: boolean
		manualSelectionMode: undefined | 'cancel' | 'rerun'
		onCancelJobs: (jobIds: string[]) => void
	}

	let {
		jobs = undefined,
		externalJobs = [],
		omittedObscuredJobs,
		showExternalJobs = false,
		selectedIds = $bindable([]),
		selectedWorkspace = $bindable(undefined),
		activeLabel = null,
		lastFetchWentToEnd = false,
		perPage = 1000,
		manualSelectionMode,
		onCancelJobs,
		batchRerunOptionsIsOpen = $bindable()
	}: Props = $props()

	let hasClickFocus = $state(false)
	const keysPressed = useKeyPressed(['Shift', 'Control', 'Meta', 'A', 'ArrowDown', 'ArrowUp'], {
		onKeyDown(key, e) {
			if (!hasClickFocus) return
			if (key === 'A' && (keysPressed.Control || keysPressed.Meta)) {
				if (batchRerunOptionsIsOpen) return
				e.preventDefault()
				e.stopPropagation()
				selectedIds = flatJobs
					? flatJobs
							.filter((jobOrDate) => jobOrDate.type === 'job')
							.map((jobOrDate) => jobOrDate.job.id)
					: []
			} else if ((key === 'ArrowDown' || key === 'ArrowUp') && selectedIds.length === 1) {
				const idx = flatJobs?.findIndex(
					(jobOrDate) => jobOrDate.type === 'job' && jobOrDate.job.id === selectedIds[0]
				)
				if (idx == undefined) return
				let nextJob = flatJobs?.[idx + (key === 'ArrowDown' ? 1 : -1)]
				if (nextJob?.type === 'date') nextJob = flatJobs?.[idx + (key === 'ArrowDown' ? 2 : -2)]
				if (nextJob?.type !== 'job') return
				selectedIds = [nextJob.job.id]
				e.preventDefault()
			}
		}
	})
	let rightClickPopover: RightClickPopover | undefined = $state(undefined)

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

	function jobCountString(
		jobCount: number | undefined,
		lastFetchWentToEnd: boolean,
		hideLabel?: boolean
	): string {
		if (jobCount === undefined) {
			return ''
		}
		const jc = jobCount
		const isTruncated = jc >= perPage && !lastFetchWentToEnd

		if (hideLabel) return `${jc}${isTruncated ? '+' : ''}`
		else return `${jc}${isTruncated ? '+' : ''} job${jc != 1 ? 's' : ''}`
	}

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

	let showTag = $derived(containerWidth > 700)
	let selectedIdsPossibleActions = $derived.by(() => {
		const cancellableJobIds: string[] = []
		const rerunnableJobIds: string[] = []
		for (const jobId of selectedIds) {
			const job = flatJobs?.find(
				(jobOrDate) => jobOrDate.type === 'job' && jobOrDate.job.id === jobId
			)
			if (job?.type === 'job') {
				if (isJobCancelable(job.job)) cancellableJobIds.push(job.job.id)
				if (isJobReRunnable(job.job)) rerunnableJobIds.push(job.job.id)
			}
		}
		return { cancellableJobIds, rerunnableJobIds }
	})
	let hoveredDropdownAction: 'cancel' | 'rerun' | null = $state(null)

	let dropdownActions: DropdownMenuProps['items'] = $derived.by(() => {
		let rerunnable = selectedIdsPossibleActions.rerunnableJobIds.length
		let cancellable = selectedIdsPossibleActions.cancellableJobIds.length
		const actions: DropdownMenuProps['items'] = []
		if (selectedIds.length === 1) {
			actions.push({
				label: 'Show run details',
				icon: ExternalLinkIcon,
				onClick: () => goto(`/run/${selectedIds[0]}`)
			})
			const job = flatJobs?.find(
				(jobOrDate) => jobOrDate.type === 'job' && jobOrDate.job.id === selectedIds[0]
			)
			if (job?.type === 'job') {
				if (job.job.job_kind === 'script') {
					actions.push({
						label: 'Go to script page',
						icon: Code2Icon,
						onClick: () => goto(`/scripts/get/${job.job.script_hash}`)
					})
				}
				if (job.job.job_kind === 'flow') {
					actions.push({
						label: 'Go to flow page',
						icon: BarsStaggered,
						onClick: () => goto(`/flows/get/${job.job.script_path}`)
					})
				}
			}
		}
		if (rerunnable)
			actions.push({
				label: 'Run again',
				icon: RefreshCwIcon,
				right: selectedIds.length >= 2 ? `${rerunnable}` : undefined,
				onClick: () => {
					selectedIds = selectedIdsPossibleActions.rerunnableJobIds
					batchRerunOptionsIsOpen = true
				},
				onHover: (hover) => (hoveredDropdownAction = hover ? 'rerun' : null)
			})
		if (cancellable)
			actions.push({
				label: 'Cancel',
				icon: CircleXIcon,
				right: selectedIds.length >= 2 ? `${cancellable}` : undefined,
				onClick: () => onCancelJobs?.(selectedIdsPossibleActions.cancellableJobIds),
				onHover: (hover) => (hoveredDropdownAction = hover ? 'cancel' : null)
			})
		return actions
	})

	function jobIsSelectable(job: Job) {
		if (
			(rightClickPopover?.isOpen() && hoveredDropdownAction === 'cancel') ||
			manualSelectionMode === 'cancel'
		)
			return isJobCancelable(job)
		if (
			(rightClickPopover?.isOpen() && hoveredDropdownAction === 'rerun') ||
			manualSelectionMode === 'rerun' ||
			batchRerunOptionsIsOpen
		)
			return isJobReRunnable(job)
		return true
	}

	let selectableJobs = $derived(jobs?.filter(jobIsSelectable) ?? [])
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="divide-y h-full flex flex-col min-w-[650px]"
	id="runs-table-wrapper"
	onclick={() => (hasClickFocus = true)}
	use:clickOutside={{ onClickOutside: () => (hasClickFocus = false) }}
	bind:clientWidth={containerWidth}
>
	<div>
		<div
			class="grid sticky top-0 w-full min-h-6 my-2 pr-4 items-end"
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
		>
			{#if manualSelectionMode}
				{@const allSelected = selectedIds.length === selectableJobs?.length}
				<div class="w-4 h-4 ml-4">
					<input
						type="checkbox"
						bind:checked={
							() => allSelected,
							() => (selectedIds = allSelected ? [] : (selectableJobs.map((j) => j.id) ?? []))
						}
					/>
				</div>
			{/if}
			<div class="text-2xs px-4 flex flex-row items-center gap-2 leading-3">
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
					{@const jobCount = jobs
						? jobCountString(jobs.length, lastFetchWentToEnd, selectedIds.length >= 2)
						: ''}
					{selectedIds.length >= 2 ? `${selectedIds.length}/` : ''}<wbr />
					{jobCount}
				{/if}
			</div>
			<div class="text-xs font-semibold leading-3">Started</div>
			<div class="text-xs font-semibold leading-3">Duration</div>
			<div class="text-xs font-semibold leading-3">Path</div>
			{#if containsLabel}
				<div class="text-xs font-semibold leading-3">Label</div>
			{/if}
			<div class="text-xs font-semibold leading-3">Triggered by</div>
			{#if showTag}
				<div class="text-xs font-semibold leading-3">Tag</div>
			{/if}
			<div> </div>
		</div>
	</div>
	<div
		bind:clientHeight={tableHeight}
		class="relative flex-1 border rounded-t-md overflow-clip bg-surface-tertiary [&>.virtual-list-wrapper::-webkit-scrollbar-track]:bg-surface-tertiary"
	>
		{#if jobs?.length == 0 && (!showExternalJobs || externalJobs?.length == 0)}
			<div class="text-xs text-secondary p-8"> No jobs found for the selected filters. </div>
		{:else}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="absolute inset-0 -mt-3"
				oncontextmenu={(e) => {
					e.preventDefault()
					rightClickPopover?.open(e)
				}}
			>
				<VirtualList
					width="100%"
					height={tableHeight}
					itemCount={flatJobs?.length ?? 3}
					itemSize={42}
					overscanCount={20}
					{stickyIndices}
					{scrollToIndex}
					scrollToAlignment="center"
				>
					{#snippet header()}{/snippet}
					{#snippet item({ index, style })}
						<div {style} class="w-full bg-surface-tertiary">
							{#if flatJobs}
								{@const jobOrDate = flatJobs[index]}
								{#if jobOrDate}
									{#if jobOrDate?.type === 'date'}
										<div
											class={twMerge(
												'border-b py-1.5 font-semibold text-xs pl-4 h-[42px] flex items-end bg-surface-tertiary'
											)}
										>
											{jobOrDate.date}
										</div>
									{:else}
										{@const selected =
											jobOrDate.job.id !== '-' && selectedIds.includes(jobOrDate.job.id)}
										{@const nonSelectable = !jobIsSelectable(jobOrDate.job)}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											class={twMerge(
												'flex flex-row items-center h-full w-full select-none transition-opacity',
												nonSelectable || (rightClickPopover?.isOpen() && !selected)
													? 'opacity-20'
													: ''
											)}
										>
											<RunRow
												{manualSelectionMode}
												{containsLabel}
												{showTag}
												job={jobOrDate.job}
												{selected}
												on:select={() => {
													const jobId = jobOrDate.job.id
													if (keysPressed.Shift && selectedIds.length > 0) {
														if (nonSelectable) return
														const lastSelectedId = selectedIds[selectedIds.length - 1]
														const lastSelectedIndex = flatJobs?.findIndex(
															(jobOrDate) =>
																jobOrDate.type === 'job' && jobOrDate.job.id === lastSelectedId
														)
														if (lastSelectedIndex != undefined && flatJobs) {
															const [start, end] =
																index < lastSelectedIndex
																	? [index, lastSelectedIndex]
																	: [lastSelectedIndex, index]
															const newSelectedIds = flatJobs
																.slice(start, end + 1)
																.filter((jobOrDate) => jobOrDate.type === 'job')
																.map((jobOrDate) => jobOrDate.job.id)
															selectedIds = Array.from(new Set([...selectedIds, ...newSelectedIds]))
														}
													} else if (
														keysPressed.Control ||
														keysPressed.Meta ||
														manualSelectionMode
													) {
														if (nonSelectable) return
														if (selectedIds.includes(jobOrDate.job.id)) {
															selectedIds = selectedIds.filter((id) => id != jobId)
														} else {
															selectedIds.push(jobId)
															selectedIds = selectedIds
														}
													} else {
														if (batchRerunOptionsIsOpen) batchRerunOptionsIsOpen = false
														if (
															selectedIds.length !== 1 ||
															selectedIds[0] !== jobOrDate.job.id ||
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
			</div>
		{/if}
	</div>
</div>

<RightClickPopover bind:this={rightClickPopover}>
	<DropdownMenu closeCallback={() => rightClickPopover?.close()} items={dropdownActions} />
</RightClickPopover>

<style>
	:global(.virtual-list-wrapper::-webkit-scrollbar) {
		width: 8px !important;
		height: 8px !important;
	}
</style>
