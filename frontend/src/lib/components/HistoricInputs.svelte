<script lang="ts">
	import { type RunnableType } from '$lib/gen/index.js'
	import { sendUserToast } from '$lib/utils.js'
	import RunningJobSchemaPicker from '$lib/components/schema/RunningJobSchemaPicker.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { useJobsLoader, type UseJobLoaderArgs } from './runs/useJobsLoader.svelte'
	import { DataTable } from '$lib/components/table'
	import HistoricList from './HistoricList.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		runnableId?: string | undefined
		runnableType?: RunnableType | undefined
		selected?: string | undefined
		showAuthor?: boolean
		placement?: 'bottom-start' | 'top-start' | 'bottom-end' | 'top-end'
		limitPayloadSize?: boolean
		searchArgs?: Record<string, any> | undefined
	}

	let {
		runnableId = undefined,
		runnableType = undefined,
		selected = $bindable(undefined),
		showAuthor = false,
		placement = 'top-end',
		limitPayloadSize = false,
		searchArgs = undefined
	}: Props = $props()

	let historicList: HistoricList | undefined = $state(undefined)
	const dispatch = createEventDispatcher()

	let hasMoreCurrentRuns = false
	let page = $state(1)

	async function handleSelected(data: any) {
		if (selected === data.id) {
			resetSelected(true)
			return
		}
		selected = data.id
		if (data.payloadData === 'WINDMILL_TOO_BIG') {
			const fullPayload = await data.getFullPayload?.()
			dispatch('select', { args: fullPayload, jobId: data.id })
		} else {
			dispatch('select', {
				args: structuredClone($state.snapshot(data.payloadData)),
				jobId: data.id
			})
		}
	}

	export function loading(): boolean {
		return jobsLoader?.loading ?? false
	}

	onDestroy(() => {
		resetSelected(false)
	})

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

	export function refresh(clearCurrentRuns: boolean = false) {
		historicList?.refresh(clearCurrentRuns)
	}

	export function resetSelected(dispatchEvent?: boolean) {
		selected = undefined
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}

	function getJobKinds(runnableType: RunnableType | undefined) {
		if (runnableType === 'FlowPath') {
			return 'flow,flowpreview'
		} else if (runnableType === 'ScriptPath') {
			return 'script,preview'
		} else if (runnableType === 'ScriptHash') {
			return 'script,preview'
		}
		return ''
	}

	let jobsLoader = useJobsLoader(
		() =>
			({
				filters: {
					show_skipped: false,
					path: runnableId,
					status: 'running',
					arg: searchArgs ? JSON.stringify(searchArgs) : ''
				},
				perPage: 5,
				jobKinds: getJobKinds(runnableType),
				syncQueuedRunsCount: false,
				refreshRate: 10000,
				currentWorkspace: $workspaceStore ?? '',
				skip: !runnableId
			}) satisfies UseJobLoaderArgs
	)
	let jobs = $derived(jobsLoader?.jobs ?? [])
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-full max-h-full min-h-0 w-full flex flex-col gap-2 relative">
	<div class="grow-0" data-schema-picker>
		<DataTable size="xs" bind:currentPage={page} hasMore={hasMoreCurrentRuns} tableFixed={true}>
			{#if jobsLoader?.loading}
				<div class="text-primary absolute top-2 right-2">
					<Loader2 class="animate-spin" size={14} />
				</div>
			{/if}
			{#if jobs.length > 0}
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
					{#if jobs.length == 5}
						<tr class="text-left text-primary text-xs w-full">
							<td class="w-full px-2" colspan="3">limited to 5 runs</td>
						</tr>
					{/if}
				</tbody>
			{:else}
				<div class="text-center text-primary text-xs py-2">No job currently running</div>
			{/if}
		</DataTable>
	</div>

	<div class="min-h-0 grow" data-schema-picker>
		<HistoricList
			bind:this={historicList}
			on:error={(e) => handleError(e.detail)}
			on:select={(e) => handleSelected(e.detail)}
			{runnableId}
			{runnableType}
			{selected}
			{showAuthor}
			{placement}
			{limitPayloadSize}
			{searchArgs}
		/>
	</div>
</div>
