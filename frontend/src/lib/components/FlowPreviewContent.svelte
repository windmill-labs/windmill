<script lang="ts">
	import { Job, JobService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import FlowJobResult from './FlowJobResult.svelte'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import SchemaForm from './SchemaForm.svelte'

	const dispatch = createEventDispatcher()

	export let flow: Flow
	export let args: Record<string, any> = {}

	let intervalId: NodeJS.Timer
	let job: Job | undefined
	let jobs = []
	let jobId: string

	$: dispatch('change', jobs)

	export async function runPreview(args: Record<string, any>) {
		intervalId && clearInterval(intervalId)

		jobId = await JobService.runFlowPreview({
			workspace: $workspaceStore ?? '',
			requestBody: {
				args,
				value: flow.value,
				path: flow.path
			}
		})
		jobs = []
		intervalId = setInterval(loadJob, 1000)
		sendUserToast(`started preview ${truncateRev(jobId, 10)}`)
	}

	async function loadJob() {
		try {
			job = await JobService.getJob({ workspace: $workspaceStore!, id: jobId })
			if (job?.type == 'CompletedJob') {
				clearInterval(intervalId)
			}
		} catch (err) {
			sendUserToast(err, true)
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})
</script>

<div class="flex flex-col space-y-4 h-screen bg-white ">
	<div class="flex flex-col space-y-4 p-6 border-b-2">
		<div class="flex justify-between">
			<h3 class="text-lg leading-6 font-bold text-gray-900">Flow Preview</h3>

			<Button color="alternative" on:click={() => dispatch('close')}>
				<Icon data={faClose} />
			</Button>
		</div>
		<SchemaForm schema={flow.schema} isValid={true} bind:args />
		<Button on:click={() => runPreview(args)} size="md">Preview</Button>
	</div>
	<div class="h-full overflow-y-auto mb-16">
		{#if job}
			<div class="w-full">
				<FlowStatusViewer {job} bind:jobs />
			</div>
			{#if `result` in job}
				<FlowJobResult {job} />
			{/if}
		{/if}
	</div>
</div>
