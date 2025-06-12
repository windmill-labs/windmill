<script module lang="ts">
	export type StepHistoryData = {
		id: string
		created_at: string
		created_by: string
		success: boolean
	}
</script>

<script lang="ts">
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, createEventDispatcher, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Pin } from 'lucide-svelte'
	import { Cell } from '$lib/components/table'
	import JobPickerLight from './JobPickerLight.svelte'

	interface Props {
		selected?: string | undefined
		moduleId?: string
		getLogs?: boolean
		mockValue?: any
		mockEnabled?: boolean
		path?: string
		staticInputs?: undefined | StepHistoryData[]
		noHistory?: 'isLoop' | 'isInsideLoop' | undefined
	}

	let {
		selected = $bindable(undefined),
		moduleId = '',
		getLogs = false,
		mockValue = undefined,
		mockEnabled = false,
		path = '',
		staticInputs = undefined,
		noHistory = undefined
	}: Props = $props()

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext') ?? {}
	const dispatch = createEventDispatcher()

	let infiniteList: InfiniteList | undefined = $state(undefined)
	let loadInputsPageFn:
		| ((page: number, perPage: number) => Promise<StepHistoryData[]>)
		| undefined = undefined

	function initLoadInputs() {
		loadInputsPageFn = async (page: number, perPage: number) => {
			if (staticInputs) return staticInputs
			const previousJobs = await JobService.listCompletedJobs({
				workspace: $workspaceStore!,
				scriptPathExact: path === '' ? $pathStore + '/' + moduleId : path,
				jobKinds: ['preview', 'script', 'flowpreview', 'flow', 'flowscript'].join(','),
				page,
				perPage
			})
			return previousJobs.map((job) => ({
				id: job.id,
				created_at: job.created_at,
				created_by: job.created_by,
				success: job.success,
				getFullJob: async () => await getJobResultAndLogs(job.id, !getLogs)
			}))
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	$effect(() => {
		staticInputs && infiniteList?.loadData('forceRefresh')
	})

	async function getJobResultAndLogs(jobId: string, noLogs: boolean) {
		try {
			const job = await JobService.getJob({
				workspace: $workspaceStore ?? '',
				id: jobId ?? '',
				noLogs
			})
			return job
		} catch (error) {
			console.error('Error getting job result and logs', error)
			return undefined
		}
	}

	$effect(() => {
		infiniteList && !noHistory && untrack(() => initLoadInputs())
	})

	function handleSelect(e: CustomEvent) {
		if (e.detail === 'extraRow') {
			if (selected === 'extraRow') {
				deselect()
				return
			}
			selected = 'extraRow'
			dispatch('select', 'mock')
			return
		}

		if (selected === e.detail.id) {
			deselect()
			return
		}
		selected = e.detail.id
		dispatch('select', { ...e.detail })
	}

	export function deselect() {
		selected = undefined
		dispatch('select', undefined)
	}
</script>

<InfiniteList
	bind:this={infiniteList}
	selectedItemId={selected}
	on:error
	on:select={handleSelect}
	rounded={false}
	noBorder
	extraRowClasses={{
		bgSelected: 'bg-blue-200 dark:bg-blue-900/40',
		bgHover: 'hover:bg-blue-100 dark:hover:bg-blue-600/40',
		class: 'bg-blue-50 dark:bg-blue-700/40'
	}}
>
	{#snippet extra_row()}
		{#if mockValue}
			<Cell wrap colspan="2">
				<div
					class="text-blue-700 dark:text-blue-100 w-full flex flex-row items-center gap-2 px-2 py-1"
				>
					<Pin size={14} />

					<span class=" grow">
						{mockEnabled ? 'Pin' : 'Last pin'}
					</span>
				</div>
			</Cell>
		{/if}
	{/snippet}
	{#snippet columns()}
		<colgroup>
			<col class="w-28" />
			<col />
		</colgroup>
	{/snippet}
	{#snippet children({ item })}
		<JobPickerLight job={item} />
	{/snippet}
	{#snippet empty()}
		<div class="text-center text-tertiary text-xs py-2 px-2">
			{noHistory === 'isLoop'
				? 'History is not available with loops.'
				: noHistory === 'isInsideLoop'
					? 'History is not available inside loops.'
					: 'No run in history for this step'}
		</div>
	{/snippet}
</InfiniteList>
