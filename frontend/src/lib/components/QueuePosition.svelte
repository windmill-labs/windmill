<script lang="ts">
	import { JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { forLater, getDbClockNow } from '$lib/forLater'
	import { displayDate } from '$lib/utils'

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

	// Bumped when a scheduled-for-later deadline passes so `isScheduledForLater`
	// re-evaluates — `forLater` reads the clock but isn't otherwise reactive, so
	// a job parked in the future would never flip to "queued" without this tick.
	let clockTick = $state(0)
	let deadlineTimeout: ReturnType<typeof setTimeout> | undefined
	let isScheduledForLater = $derived.by(() => {
		clockTick
		return scheduledFor != undefined && forLater(scheduledFor)
	})

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

	// Flip out of the "scheduled for later" state exactly when the deadline
	// passes, so a job whose scheduled time arrives with no free worker then
	// correctly surfaces its queue position.
	$effect(() => {
		clearTimeout(deadlineTimeout)
		if (scheduledFor != undefined) {
			const ms = new Date(scheduledFor).getTime() - 5000 - getDbClockNow().getTime()
			if (ms > 0) {
				deadlineTimeout = setTimeout(() => clockTick++, ms)
			}
		}
		return () => clearTimeout(deadlineTimeout)
	})

	$effect(() => {
		// Only poll the queue position for jobs actually waiting for a worker —
		// a job scheduled for the future isn't "in line", so its position is
		// meaningless (and the query counts every job scheduled before it).
		if (scheduledFor && !isScheduledForLater) {
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

{#if isScheduledForLater && scheduledFor != undefined}
	<div class="text-xs ml-4">
		<span class="text-blue-600">Scheduled for <b>{displayDate(new Date(scheduledFor))}</b></span>
	</div>
{:else if queueState}
	<div class="text-xs ml-4">
		<span class="text-orange-600">Queue position: <b>{queueState.position}</b></span>
		{#if !minimal}
			<span class="ml-2 text-primary">(Waiting for an available worker)</span>
		{/if}
	</div>
{/if}
