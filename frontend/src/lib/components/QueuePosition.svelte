<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	let { jobId, workspaceId }: { jobId: string; workspaceId?: string | undefined } = $props()

	let queuePositionInterval: NodeJS.Timeout | undefined
	let queueState = $state(undefined) as undefined | { position?: number }

	let fetchingQueuePosition = false

	let workspace = $derived(workspaceId ?? $workspaceStore)

	$effect(() => {
		// Fetch queue position when loading and we have jobId
		if (jobId && workspace) {
			// Initial fetch
			fetchQueuePosition()

			// Set up interval to refresh every 2 seconds
			queuePositionInterval = setInterval(() => {
				fetchQueuePosition()
			}, 2000)
		} else {
			// Clear interval when not loading
			if (queuePositionInterval) {
				clearInterval(queuePositionInterval)
				queuePositionInterval = undefined
			}
			queueState = undefined
		}

		return () => {
			if (queuePositionInterval) {
				clearInterval(queuePositionInterval)
			}
		}
	})

	async function fetchQueuePosition() {
		if (!jobId || !workspace || fetchingQueuePosition) return

		try {
			fetchingQueuePosition = true
			const response = await JobService.getQueuePosition({
				workspace: workspace,
				id: jobId
			})

			queueState = response
		} catch (error) {
			console.error('Failed to fetch queue position:', error)
			queueState = undefined
		} finally {
			fetchingQueuePosition = false
		}
	}
</script>

{#if queueState}
	<span class="text-small text-secondary"
		>Waiting for an available worker. Queue position: <span class="font-bold"
			>{queueState.position}</span
		></span
	>
{/if}
