<script lang="ts">
	import { InputService, type Input, type RunnableType, type Job } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import JobLoader from './runs/JobLoader.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null

	const dispatch = createEventDispatcher()
	const perPage = 10

	let previousInputs: Input[] | undefined = undefined
	let jobs: Job[] = []
	let loading: boolean = false
	let hasMoreCurrentRuns = false

	$: runnableId = scriptHash || scriptPath || flowPath || undefined

	let runnableType: RunnableType | undefined = undefined
	$: runnableType = scriptHash
		? 'ScriptHash'
		: scriptPath
		? 'ScriptPath'
		: flowPath
		? 'FlowPath'
		: undefined

	let hasAlreadyFailed = false
	async function loadInputHistory() {
		try {
			previousInputs = await InputService.getInputHistory({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load input history: ${e}`, true)
		}
	}

	$: {
		if ($workspaceStore && (scriptHash || scriptPath || flowPath)) {
			loadInputHistory()
		}
	}

	function handleSelected(data: any) {
		if (selected === data.jobId) {
			selected = undefined
			dispatch('select', undefined)
			return
		}
		selected = data.jobId
		dispatch('select', data.payloadData)
	}

	let selected: string | undefined = undefined

	onDestroy(() => {
		selected = undefined
		dispatch('select', undefined)
	})

	$: hasMorePreviousRuns = previousInputs ? previousInputs.length === perPage : false

	let page = 1
	function goToNextPage() {
		page++
		loadInputHistory()
	}

	function goToPreviousPage() {
		page--
		loadInputHistory()
	}
</script>

<JobLoader
	bind:jobs
	path={runnableId ?? null}
	isSkipped={false}
	jobKindsCat="jobs"
	jobKinds="all"
	user={null}
	label={null}
	folder={null}
	concurrencyKey={null}
	tag={null}
	success="running"
	argFilter={undefined}
	bind:loading
	syncQueuedRunsCount={false}
	refreshRate={10000}
	computeMinAndMax={undefined}
	perPage={5}
/>

<div class="h-full">
	<div class="w-full flex flex-col gap-4">
		<DataTable
			size="xs"
			on:next={goToNextPage}
			on:previous={goToPreviousPage}
			bind:currentPage={page}
			hasMore={hasMoreCurrentRuns}
		>
			{#if loading && (jobs == undefined || jobs?.length == 0)}
				<div class="text-center text-tertiary text-xs py-2">Loading current runs...</div>
			{:else if jobs?.length > 0}
				<colgroup>
					<col class="w-8" />
					<col class="w-20" />
					<col />
				</colgroup>

				<tbody class="w-full overflow-y-auto">
					{#each jobs as job (job.id)}
						<JobSchemaPicker
							runningJob={true}
							{job}
							selected={selected === job.id}
							on:select={(e) => handleSelected(e.detail)}
						/>
					{/each}
					{#if jobs?.length == 5}
						<div class="text-left text-tertiary text-xs"
							>... there may be more runs not displayed here as the limit is 5</div
						>
					{/if}
				</tbody>
			{:else}
				<div class="text-center text-tertiary text-xs py-2">No running runs</div>
			{/if}
		</DataTable>

		<DataTable
			size="xs"
			paginated={page > 1 || (previousInputs && previousInputs.length > 0)}
			on:next={goToNextPage}
			on:previous={goToPreviousPage}
			bind:currentPage={page}
			hasMore={hasMorePreviousRuns}
		>
			{#if previousInputs === undefined}
				<Skeleton layout={[[1], 0.5, [1]]} />
			{:else if previousInputs?.length > 0}
				<colgroup>
					<col class="w-8" />
					<col class="w-20" />
					<col />
				</colgroup>

				<tbody class="w-full overflow-y-auto">
					{#each previousInputs as job (job.id)}
						<JobSchemaPicker
							{job}
							selected={selected === job.id}
							on:select={(e) => handleSelected(e.detail)}
						/>
					{/each}
				</tbody>
			{:else if page > 1}
				<div class="text-center text-tertiary text-xs py-2">No more previous Runs</div>
			{:else}
				<div class="text-center text-tertiary text-xs py-2">No previous Runs</div>
			{/if}
		</DataTable>
	</div>
</div>
