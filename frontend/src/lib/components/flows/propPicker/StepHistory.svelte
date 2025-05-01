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
	import { getContext, createEventDispatcher } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Pin } from 'lucide-svelte'
	import { Cell } from '$lib/components/table'
	import JobPickerLight from './JobPickerLight.svelte'

	export let selected: string | undefined = undefined
	export let moduleId: string = ''
	export let getLogs: boolean = false
	export let mockValue: any = undefined
	export let mockEnabled: boolean = false
	export let path: string = ''
	export let staticInputs: undefined | StepHistoryData[] = undefined
	export let noHistory: 'isLoop' | 'isInsideLoop' | undefined = undefined

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext') ?? {}
	const dispatch = createEventDispatcher()

	let infiniteList: InfiniteList | undefined = undefined
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

	$: staticInputs && infiniteList?.loadData('forceRefresh')

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

	$: infiniteList && !noHistory && initLoadInputs()

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
	<svelte:fragment slot="extra-row">
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
	</svelte:fragment>
	<svelte:fragment slot="columns">
		<colgroup>
			<col class="w-28" />
			<col />
		</colgroup>
	</svelte:fragment>
	<svelte:fragment let:item>
		<JobPickerLight job={item} />
	</svelte:fragment>
	<svelte:fragment slot="empty">
		<div class="text-center text-tertiary text-xs py-2 px-2">
			{noHistory === 'isLoop'
				? 'History is not available with loops.'
				: noHistory === 'isInsideLoop'
					? 'History is not available inside loops.'
					: 'No run in history for this step'}
		</div>
	</svelte:fragment>
</InfiniteList>
