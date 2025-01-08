<script lang="ts">
	import { InputService, type Input, type RunnableType, type Job } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import JobLoader from './runs/JobLoader.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import { clickOutside } from '$lib/utils'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null

	const dispatch = createEventDispatcher()
	const perPage = 10

	let previousInputs: Input[] = []
	let jobs: Job[] = []
	let loading: boolean = false
	let hasMoreCurrentRuns = false
	let hasMorePreviousRuns = false
	let page = 1

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
	async function loadInputHistory(refresh = false) {
		hasMorePreviousRuns = false
		if (refresh) {
			previousInputs = []
			page = 1
			return loadInputHistory(false)
		}
		try {
			const newInputs = await InputService.getInputHistory({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage
			})
			previousInputs = [...previousInputs, ...newInputs]
			hasMorePreviousRuns = previousInputs ? previousInputs.length === perPage * page : false
			page++
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

	async function getPropPickerElements(): Promise<HTMLElement[]> {
		return Array.from(
			document.querySelectorAll('[data-schema-picker], [data-schema-picker] *')
		) as HTMLElement[]
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			selected = undefined
			dispatch('select', undefined)
		}
	}
</script>

<svelte:window on:keydown={handleKeydown} />

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

<div
	class="h-full w-full flex flex-col gap-4"
	use:clickOutside={{ capture: false, exclude: getPropPickerElements }}
	on:click_outside={() => {
		if (selected) {
			selected = undefined
			dispatch('select', undefined)
		}
	}}
>
	<div class="grow-0">
		<DataTable size="xs" bind:currentPage={page} hasMore={hasMoreCurrentRuns} tableFixed={true}>
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
	</div>

	<div class="min-h-0 grow">
		<DataTable
			size="xs"
			on:loadMore={() => loadInputHistory()}
			infiniteScroll
			hasMore={hasMorePreviousRuns}
			tableFixed={true}
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
			{:else if page > 2}
				<div class="text-center text-tertiary text-xs py-2">No more previous Runs</div>
			{:else}
				<div class="text-center text-tertiary text-xs py-2">No previous Runs</div>
			{/if}
		</DataTable>
	</div>
</div>
