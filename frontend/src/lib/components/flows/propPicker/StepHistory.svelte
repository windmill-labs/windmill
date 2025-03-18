<script lang="ts">
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, createEventDispatcher } from 'svelte'
	import type { FlowEditorContext } from '../types'

	export let selected: string | undefined = undefined
	export let moduleId: string = ''

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
					const result = await getJobResult(job.id)
					return {
						...job,
						payloadData: result
					}
				})
			)
			return jobsResults
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	async function getJobResult(jobId: string) {
		const job = await JobService.getJob({
			workspace: $workspaceStore ?? '',
			id: jobId ?? '',
			noLogs: true
		})
		// Handle different job types
		if ('result' in job) {
			return job.result ?? {}
		}
		return {}
	}

	$: infiniteList && initLoadInputs()

	function handleselect(e: CustomEvent) {
		if (selected === e.detail.id) {
			selected = undefined
			dispatch('select', undefined)
			return
		}
		selected = e.detail.id
		dispatch('select', { result: e.detail.payloadData, jobId: e.detail.id })
	}
</script>

<InfiniteList bind:this={infiniteList} selectedItemId={selected} on:error on:select={handleselect}>
	<svelte:fragment slot="columns">
		<colgroup>
			<col class="w-8" />
			<col class="w-16" />
			<col />
		</colgroup>
	</svelte:fragment>
	<svelte:fragment let:item let:hover>
		<JobSchemaPicker job={item} hovering={hover} payloadData={item.payloadData} />
	</svelte:fragment>
	<svelte:fragment slot="empty">
		<div class="text-center text-tertiary text-xs py-2"> 'No run yet' </div>
	</svelte:fragment>
</InfiniteList>
