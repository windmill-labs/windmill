<script lang="ts">
	import { Job, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast, truncateRev } from '$lib/utils'
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Icon from 'svelte-awesome'
	import FlowJobResult from './FlowJobResult.svelte'
	import { flowStateStore, flowStateToFlow } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import { runFlowPreview } from './flows/utils'
	import FlowStatusViewer from './FlowStatusViewer.svelte'
	import SchemaForm from './SchemaForm.svelte'

	export let args: Record<string, any> = {}

	let intervalId: NodeJS.Timer
	let job: Job | undefined
	let jobs: (Job | Job[])[] = []
	let jobId: string
	let isValid: boolean = false

	const dispatch = createEventDispatcher()

	export async function runPreview(args: Record<string, any>) {
		intervalId && clearInterval(intervalId)
		const newFlow = flowStateToFlow($flowStateStore, $flowStore)

		jobId = await runFlowPreview(args, newFlow)
		jobs = []
		intervalId = setInterval(loadJob, 1000)
		sendUserToast(`started preview ${truncateRev(jobId, 10)}`)
	}

	$: {
		const resultsSoFar: any[] = []
		jobs.map((x, i) => {
			if (Array.isArray(x)) {
				const innerResults: any[] = []
				x.forEach((y, j) => {
					if (`result` in y) {
						innerResults.push(y.result)
						if ($flowStateStore[i].childFlowModules![j] != undefined) {
							$flowStateStore[i].childFlowModules![j].previewResult = JSON.parse(
								JSON.stringify(innerResults)
							)
						}
					}
				})
				resultsSoFar.push(innerResults)
			} else if (`result` in x) {
				resultsSoFar.push(x.result)
			}
			$flowStateStore[i].previewResult = JSON.parse(JSON.stringify(resultsSoFar))
		})
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

<div class="flex flex-col space-y-4 h-screen bg-white">
	<div class="flex flex-col space-y-4 p-6 border-b-2 overflow-y-auto grow">
		<div class="flex justify-between">
			<h3 class="text-lg leading-6 font-bold text-gray-900">Flow Preview</h3>

			<Button color="alternative" on:click={() => dispatch('close')}>
				<Icon data={faClose} />
			</Button>
		</div>
		<SchemaForm schema={$flowStore.schema} bind:isValid bind:args />
	</div>
	<Button disabled={!isValid} class="blue-button  mx-4" on:click={() => runPreview(args)} size="md">
		Preview
	</Button>
	<div class="h-full overflow-y-auto mb-16 grow">
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
