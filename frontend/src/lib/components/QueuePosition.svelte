<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	let {
		jobId,
		workspaceId,
		minimal = false
	}: { jobId: string; workspaceId?: string | undefined; minimal?: boolean } = $props()

	let queuePositionInterval: number | undefined
	let queueState = $state(undefined) as undefined | { position?: number }

	let fetchingQueuePosition = false

	let workspace = $derived(workspaceId ?? $workspaceStore)

	let scheduledFor = $state(undefined) as undefined | number

	let scheduledForTimeout: number | undefined
	$effect(() => {
		if (jobId && workspace) {
			clearTimeout(scheduledForTimeout)
			queueState = undefined
			scheduledForTimeout = setTimeout(() => {
				try {
					JobService.getScheduledFor({
						workspace: workspace,
						id: jobId
					})
						.then((response) => {
							scheduledFor = response
						})
						.catch((error) => {
							console.error('Failed to fetch scheduled for:', error)
						})
				} catch (error) {
					console.error('Failed to fetch scheduled for:', error)
				}
			}, 2000)
		}
	})

	$effect(() => {
		// Fetch queue position when loading and we have jobId
		if (scheduledFor) {
			// Initial fetch
			fetchQueuePosition()

			// Set up interval to refresh every 2 seconds
			queuePositionInterval = setInterval(() => {
				fetchQueuePosition()
			}, 5000)
		} else {
			// Clear interval when not loading
			if (queuePositionInterval) {
				clearInterval(queuePositionInterval)
				queuePositionInterval = undefined
			}
			queueState = undefined
		}

		return () => {
			scheduledForTimeout && clearTimeout(scheduledForTimeout)
			if (queuePositionInterval) {
				clearInterval(queuePositionInterval)
			}
		}
	})

	async function fetchQueuePosition() {
		if (!workspace || !scheduledFor || fetchingQueuePosition) return

		try {
			fetchingQueuePosition = true
			queueState = await JobService.getQueuePosition({
				workspace: workspace,
				scheduledFor: scheduledFor
			})
		} catch (error) {
			console.error('Failed to fetch queue position:', error)
			queueState = undefined
		} finally {
			fetchingQueuePosition = false
		}
	}
</script>

{#if queueState}
	<div class="text-xs ml-4">
		<span class="text-orange-600">Queue position: <b>{queueState.position}</b></span>
		{#if !minimal}
			<span class="ml-2 text-primary">(Waiting for an available worker)</span>
		{/if}
	</div>
{/if}
