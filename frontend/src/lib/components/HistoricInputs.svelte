<script lang="ts">
	import { InputService, type RunnableType, type Job } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { sendUserToast } from '$lib/utils.js'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import RunningJobSchemaPicker from '$lib/components/schema/RunningJobSchemaPicker.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import JobLoader from './runs/JobLoader.svelte'
	import { DataTable } from '$lib/components/table'
	import InfiniteList from './InfiniteList.svelte'

	export let runnableId: string | undefined = undefined
	export let runnableType: RunnableType | undefined = undefined
	export let loading: boolean = false

	const dispatch = createEventDispatcher()

	let jobs: Job[] = []
	let hasMoreCurrentRuns = false
	let page = 1
	let infiniteList: InfiniteList | undefined = undefined
	let loadInputsPageFn: ((page: number, perPage: number) => Promise<any>) | undefined = undefined

	let cachedArgs: Record<string, any> = {}
	function initLoadInputs() {
		loadInputsPageFn = async (page: number, perPage: number) => {
			const inputs = await InputService.getInputHistory({
				workspace: $workspaceStore!,
				runnableId,
				runnableType,
				page,
				perPage,
				includePreview: true
			})

			const inputsWithPayload = await Promise.all(
				inputs.map(async (input) => {
					if (cachedArgs[input.id]) {
						return {
							...input,
							payloadData: cachedArgs[input.id]
						}
					}
					const payloadData = await loadArgsFromHistory(input.id, undefined, false)
					if (payloadData === 'WINDMILL_TOO_BIG') {
						return {
							...input,
							payloadData: 'WINDMILL_TOO_BIG',
							getFullPayload: () => loadArgsFromHistory(input.id, undefined, true)
						}
					}
					cachedArgs[input.id] = payloadData
					return {
						...input,
						payloadData
					}
				})
			)
			return inputsWithPayload
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	async function handleSelected(data: any) {
		if (selected === data.id) {
			resetSelected(true)
			return
		}
		selected = data.id
		if (data.payloadData === 'WINDMILL_TOO_BIG') {
			const fullPayload = await data.getFullPayload?.()
			dispatch('select', fullPayload)
		} else {
			dispatch('select', structuredClone(data.payloadData))
		}
	}

	let selected: string | undefined = undefined

	onDestroy(() => {
		resetSelected(true)
	})

	async function loadArgsFromHistory(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean
	): Promise<any> {
		if (!id) return
		const payloadData = await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: $workspaceStore!,
			input,
			allowLarge
		})
		return payloadData
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && selected) {
			resetSelected(true)
			event.stopPropagation()
			event.preventDefault()
		}
	}

	function handleError(e: { type: string; error: any }) {
		if (e.type === 'load') {
			sendUserToast(`Failed to load input history: ${e.error}`, true)
		}
	}

	let jobHovered: string | undefined = undefined

	export function refresh() {
		if (infiniteList) {
			infiniteList.loadData('refresh')
		}
	}

	export function resetSelected(dispatchEvent?: boolean) {
		console.log('resetSelected')
		selected = undefined
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}

	$: !loading && refresh()
	$: $workspaceStore && runnableId && runnableType && infiniteList && initLoadInputs()
</script>

<svelte:window on:keydown={handleKeydown} />

{#if runnableId}
	<JobLoader
		bind:jobs
		path={runnableId}
		isSkipped={false}
		jobKindsCat="all"
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
{/if}

<div class="h-full w-full flex flex-col gap-4">
	<div class="grow-0" data-schema-picker>
		<DataTable size="xs" bind:currentPage={page} hasMore={hasMoreCurrentRuns} tableFixed={true}>
			{#if loading && (jobs == undefined || jobs?.length == 0)}
				<div class="text-center text-tertiary text-xs py-2">Loading current runs...</div>
			{:else if jobs?.length > 0}
				<colgroup>
					<col class="w-8" />
					<col class="w-16" />
					<col />
				</colgroup>

				<tbody class="w-full overflow-y-auto">
					{#each jobs as job (job.id)}
						<RunningJobSchemaPicker
							{job}
							selected={selected === job.id}
							hovering={jobHovered === job.id}
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
				<div class="text-center text-tertiary text-xs py-2">No job currently running</div>
			{/if}
		</DataTable>
	</div>

	<div class="min-h-0 grow" data-schema-picker>
		<InfiniteList
			bind:this={infiniteList}
			selectedItemId={selected}
			on:error={(e) => handleError(e.detail)}
			on:select={(e) => handleSelected(e.detail)}
		>
			<svelte:fragment slot="columns">
				<colgroup>
					<col class="w-8" />
					<col class="w-16" />
					<col />
				</colgroup>
			</svelte:fragment>
			<svelte:fragment let:item let:hover>
				<JobSchemaPicker
					job={item}
					selected={selected === item.id}
					hovering={hover}
					payloadData={item.payloadData}
				/>
			</svelte:fragment>
			<svelte:fragment slot="empty">
				<div class="text-center text-tertiary text-xs py-2">
					{runnableId ? 'No previous inputs' : 'Save draft to see previous runs'}
				</div>
			</svelte:fragment>
		</InfiniteList>
	</div>
</div>
