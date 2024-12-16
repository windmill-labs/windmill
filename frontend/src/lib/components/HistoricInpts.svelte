<script lang="ts">
	import { InputService, type Input, type RunnableType, type Job } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import type { Capture } from '$lib/gen'
	import { CaptureService } from '$lib/gen'
	import { sendUserToast } from '$lib/utils.js'
	import { isObject } from '$lib/utils'
	import JobSchemaPicker from '$lib/components/schema/JobSchemaPicker.svelte'
	import SchemaPicker from '$lib/components/schema/SchemaPicker.svelte'

	import JobLoader from './runs/JobLoader.svelte'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import { _ } from 'ag-grid-community'

	export let scriptHash: string | null = null
	export let scriptPath: string | null = null
	export let flowPath: string | null = null
	export let isFlow = false
	export let testKind: 'main' | 'preprocessor' = 'main'
	export let hasPreprocessor = false

	$: path = isFlow ? flowPath : scriptPath
	$: console.log('dbg path', path)

	interface EditableInput extends Input {
		isEditing?: boolean
		isSaving?: boolean
	}

	let previousInputs: Input[] | undefined = undefined
	let savedInputs: EditableInput[] | undefined = undefined
	let jobs: Job[] = []
	let loading: boolean = false

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
				perPage: 10
			})
		} catch (e) {
			console.error(e)
			if (hasAlreadyFailed) return
			hasAlreadyFailed = true
			sendUserToast(`Failed to load input history: ${e}`, true)
		}
	}

	async function loadSavedInputs() {
		savedInputs = await InputService.listInputs({
			workspace: $workspaceStore!,
			runnableId,
			runnableType,
			perPage: 10
		})
	}

	$: {
		if ($workspaceStore && (scriptHash || scriptPath || flowPath)) {
			loadInputHistory()
			loadSavedInputs()
		}
	}

	let captures: Capture[] = []
	async function refreshCaptures() {
		console.log('dbg refreshCaptures', captures)
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path: path ?? ''
		})
	}
	$: path && refreshCaptures()

	$: console.log('dbg captures', captures)
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

<div class="min-w-[300px] h-full">
	<div class="w-full flex flex-col gap-4 p-2">
		<div class="w-full flex flex-col gap-1 p-0 h-full overflow-y-auto">
			{#if loading && (jobs == undefined || jobs?.length == 0)}
				<div class="text-left text-tertiary text-xs">Loading current runs...</div>
			{:else if jobs?.length > 0}
				{#each jobs as job (job.id)}
					<JobSchemaPicker {job} {isFlow} on:updateSchema on:applyArgs />
				{/each}
				{#if jobs?.length == 5}
					<div class="text-left text-tertiary text-xs"
						>... there may be more runs not displayed here as the limit is 5</div
					>
				{/if}
			{/if}
		</div>

		<div class="w-full flex flex-col gap-1 p-0 h-full overflow-y-auto">
			{#if previousInputs === undefined}
				<Skeleton layout={[[8]]} />
			{:else if previousInputs?.length > 0}
				{#each previousInputs as job (job.id)}
					<JobSchemaPicker {job} {isFlow} on:updateSchema on:applyArgs />
				{/each}
			{:else}
				<div class="text-center text-tertiary">No previous Runs</div>
			{/if}

			<div class="flex flex-col gap-1 pt-2 grow overflow-y-auto">
				{#if captures.length === 0}
					<div class="text-xs text-secondary">No captures yet</div>
				{:else}
					{#each captures as capture}
						{@const payload = isObject(capture.payload) ? capture.payload : {}}
						{@const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}}
						{@const payloadData =
							testKind === 'preprocessor'
								? {
										...payload,
										...triggerExtra
								  }
								: payload}
						<SchemaPicker
							date={capture.created_at}
							{payloadData}
							{testKind}
							{isFlow}
							canEdit={false}
							deleteLoading={false}
							{hasPreprocessor}
							allowApplyArgs={false}
							on:updateSchema
							on:applyArgs
							on:addPreprocessor
						/>
					{/each}
				{/if}
			</div>
		</div>
	</div>
</div>
