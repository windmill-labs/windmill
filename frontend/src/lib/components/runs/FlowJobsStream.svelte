<script lang="ts">
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onMount } from 'svelte'

	let { jobId } = $props()

	let payloads: {
		flow_step_id: string
		content: string
	}[] = $state([])

	function processStreamUpdate({
		flow_step_id,
		only_result,
		new_result_stream
	}: {
		flow_step_id?: string
		only_result?: string
		new_result_stream?: string
	}) {
		if (flow_step_id) {
			if (only_result) {
				// only show only_result if we haven't been streaming it before
				const existingPayload = payloads.find((p) => p.flow_step_id === flow_step_id)
				if (!existingPayload) {
					payloads.push({ flow_step_id, content: only_result })
				}
			} else if (new_result_stream) {
				const lastPayload = payloads[payloads.length - 1]

				if (lastPayload && flow_step_id === lastPayload.flow_step_id) {
					lastPayload.content += new_result_stream
				} else {
					payloads.push({ flow_step_id, content: new_result_stream })
				}
			}
		}
	}

	async function getFlowJobsStream() {
		const sseUrl = `/api/w/${$workspaceStore}/jobs_u/getupdate_sse/${jobId}?only_jobs_stream=true`

		const eventSource = new EventSource(sseUrl)

		eventSource.onmessage = (event) => {
			const data = JSON.parse(event.data)

			switch (data.type) {
				case 'timeout':
					eventSource.close()
					break
				case 'error':
					eventSource.close()
					console.error('Error during stream of flow jobs', data)
					sendUserToast('Error during stream of flow jobs', true)
					break
				case 'not_found':
					console.error('Job not found', data)
					eventSource.close()
					sendUserToast('Job not found', true)
					break
				case 'update':
					processStreamUpdate(data)
					break
				case 'ping':
					break
				default:
					console.warn('Unknown SSE event: ' + data)
			}
		}

		eventSource.onerror = (error) => {
			console.warn('SSE error', error)
			eventSource.close()
		}

		eventSource.onopen = () => {
			console.log('SSE connection opened for flow jobs streaming of flow job:', jobId)
		}
	}

	onMount(() => {
		getFlowJobsStream()
	})
</script>

<div class="flex flex-col gap-2">
	{#each payloads as payload}
		{@const [stepId, stepJobId] = payload.flow_step_id.split(':')}
		<div class="flex flex-col gap-1">
			<a href={`${base}/run/${stepJobId}`} target="_blank">Step {stepId}</a>
			<pre class="whitespace-pre-wrap text-sm">{payload.content}</pre>
		</div>
	{/each}
</div>
