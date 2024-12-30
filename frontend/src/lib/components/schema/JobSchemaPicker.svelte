<script lang="ts">
	import SchemaPickerRow from './SchemaPickerRow.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import { InputService, JobService } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { base } from '$lib/base'
	import Row from '$lib/components/table/Row.svelte'
	import Cell from '$lib/components/table/Cell.svelte'

	export let job: any
	export let selected = false
	export let payloadData: any | undefined = undefined
	const dispatch = createEventDispatcher()
	export let runningJob = false

	if (runningJob) {
		loadArgsFromRunningJob(job.id)
	} else {
		loadArgsFromHistoryOrSavedInput(job.id, undefined, false)
	}

	async function loadArgsFromHistoryOrSavedInput(
		id: string | undefined,
		input: boolean | undefined,
		allowLarge: boolean
	): Promise<any> {
		if (!id) return
		payloadData = await InputService.getArgsFromHistoryOrSavedInput({
			jobOrInputId: id,
			workspace: $workspaceStore!,
			input,
			allowLarge
		})
		loadingArgs = false
	}

	async function loadArgsFromRunningJob(id: string | undefined) {
		if (!id) return
		payloadData = await JobService.getJobArgs({
			workspace: $workspaceStore!,
			id
		})
		loadingArgs = false
	}

	let loadingArgs = true
</script>

{#if loadingArgs}
	<Row>
		<Cell>
			<Skeleton layout={[[1]]} />
		</Cell>
		<Cell>
			<Skeleton layout={[[1]]} />
		</Cell>
		<Cell>
			<Skeleton layout={[[1]]} />
		</Cell>
	</Row>
{:else}
	<SchemaPickerRow
		{payloadData}
		on:updateSchema
		on:applyArgs={async () => {
			dispatch('selected_args', payloadData)
		}}
		on:select={() => dispatch('select', { jobId: job.id, payloadData })}
		date={job.created_at}
		{selected}
	>
		<svelte:fragment slot="start">
			<div class="center-center">
				<div
					class="rounded-full w-2 h-2 {runningJob
						? 'bg-orange-400 animate-pulse'
						: job.success
						? 'bg-green-400'
						: 'bg-red-400'}"
					title={runningJob ? 'Running' : job.success ? 'Success' : 'Failed'}
				/>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="extra">
			<div class="center-center">
				<a
					target="_blank"
					href="{base}/run/{job.id}?workspace={$workspaceStore}"
					class="text-right float-right text-secondary"
					title="See run detail in a new tab"
				>
					<ExternalLink size={16} />
				</a>
			</div>
		</svelte:fragment>
	</SchemaPickerRow>
{/if}
