<script lang="ts">
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, createEventDispatcher } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Pin } from 'lucide-svelte'
	import { Cell } from '$lib/components/table'

	export let selected: string | undefined = undefined
	export let moduleId: string = ''
	export let getLogs: boolean = false
	export let mockValue: any = undefined
	export let mockEnabled: boolean = false

	const { pathStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	let infiniteList: InfiniteList | undefined = undefined
	let loadInputsPageFn: ((page: number, perPage: number) => Promise<any>) | undefined = undefined

	function initLoadInputs() {
		loadInputsPageFn = async (page: number, perPage: number) => {
			const previousJobs = await JobService.listJobs({
				workspace: $workspaceStore!,
				scriptPathExact: $pathStore + '/' + moduleId,
				jobKinds: ['preview', 'script', 'flowpreview', 'flow'].join(','),
				page: 1,
				perPage: 10
			})
			const jobsResults = await Promise.all(
				previousJobs.map(async (job) => {
					const fullJob = await getJobResultAndLogs(job.id, !getLogs)
					return fullJob
				})
			)
			return jobsResults
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	async function getJobResultAndLogs(jobId: string, noLogs: boolean) {
		const job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? '',
			noLogs
		})
		return job
	}

	$: infiniteList && initLoadInputs()

	function handleselect(e: CustomEvent) {
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
	on:select={handleselect}
	rounded={false}
	noBorder
	extraRowClasses={{
		bgSelected: 'bg-blue-200',
		bgHover: 'hover:bg-blue-100',
		class: 'bg-blue-50'
	}}
>
	<svelte:fragment slot="extra-row">
		{#if mockValue && !mockEnabled}
			<Cell>
				<div class="center-center">
					<Pin size={14} class="text-blue-700 dark:text-blue-100" />
				</div>
			</Cell>
			<Cell colspan="2" wrap>
				<span class="text-blue-700 dark:text-blue-100"> Previous pinned data </span>
			</Cell>
		{/if}
	</svelte:fragment>
	<svelte:fragment slot="columns">
		<colgroup>
			<col class="w-8" />
			<col class="w-16" />
			<col />
		</colgroup>
	</svelte:fragment>
	<svelte:fragment let:item let:hover>
		<JobSchemaPicker job={item} hovering={hover} payloadData={item.result} />
	</svelte:fragment>
	<svelte:fragment slot="empty">
		<div class="text-center text-tertiary text-xs py-2"> 'No run yet' </div>
	</svelte:fragment>
</InfiniteList>
