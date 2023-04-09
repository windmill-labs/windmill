<script lang="ts">
	import { Button } from '$lib/components/common'
	import { JobService, type JobInput } from '$lib/gen/index.js'
	import { workspaceStore } from '$lib/stores.js'
	import { displayDate } from '$lib/utils.js'
	import { createEventDispatcher } from 'svelte'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'

	export let hash: string | undefined = ''
	export let path: string | undefined = ''
	let job_inputs: JobInput[] = []
	let selected_args = {}

	async function loadInputHistory() {
		if (hash) {
			job_inputs = await JobService.getInputHistoryByHash({
				workspace: $workspaceStore!,
				hash,
				perPage: 10
			})
		} else if (path) {
			job_inputs = await JobService.getInputHistoryByPath({
				workspace: $workspaceStore!,
				path,
				perPage: 10
			})
		}
	}

	$: {
		if ($workspaceStore && (hash || path)) {
			loadInputHistory()
		}
	}

	const dispatch = createEventDispatcher()

	const selectArgs = (selected_args: object) => {
		dispatch('selected_args', selected_args)
	}
</script>

<div class="fixed right-0 top-0 bottom-0 w-80 h-full grid grid-rows-2 bg-gray-50 border-l">
	<div class="w-full flex flex-col gap-4 p-4">
		<h2>Previous Inputs</h2>

		<div class="w-full flex flex-col gap-2 p-2 h-full overflow-y-auto overflow-x-hidden">
			{#if job_inputs.length > 0}
				{#each job_inputs as { created_by, started_at, args }}
					<Button color="blue" btnClasses="w-full" on:click={() => (selected_args = args)}>
						<div class="w-full h-full items-center flex gap-4">
							<small>{displayDate(started_at)}</small>
							<small class="w-[160px] overflow-x-hidden text-ellipsis text-left">
								{created_by}
							</small>
						</div>
					</Button>
				{/each}
			{:else}
				<div class="text-center text-gray-500">No previous inputs</div>
			{/if}
		</div>
	</div>

	<div class="w-full flex flex-col gap-4 p-4 border-t">
		<h2>Preview</h2>

		<div class="w-full h-full overflow-auto">
			{#if Object.keys(selected_args).length > 0}
				<ObjectViewer json={selected_args} />
			{:else}
				<div class="text-center text-gray-500">Select an Input to preview scripts arguments</div>
			{/if}
		</div>
	</div>

	<div class="w-full flex flex-col p-4 border-t">
		<Button
			color="blue"
			btnClasses="w-full"
			on:click={() => selectArgs(selected_args)}
			disabled={Object.keys(selected_args).length === 0}
		>
			Use Input
		</Button>
	</div>
</div>
