<script lang="ts">
	import { type Job } from '$lib/gen'
	import { isScriptPreview } from '$lib/utils'
	import { onDestroy } from 'svelte'

	export let job: Job | undefined = undefined
	/** Execution duration of current active job (in ms) */
	export let executionDuration: number = 0
	/** Is current job running more than specified value in `longDefinition` seconds */
	export let longRunning: boolean = false
	/** What do we count as "long" (in ms)*/
	export let longDefinition: number = 30_000
	/** How often component updates execution duration (in ms)
	 *   Higher value -> more efficient component is, less accuracy it has
	 *   Lower value -> less efficient component is, more accuracy it has
	 */
	export let updateResolution: number = 5_000

	let startedAt: number | undefined = undefined
	let busy: boolean = false
	let interval: number | undefined
	// Detect when execution of job started
	$: if (
		!busy &&
		job &&
		'running' in job &&
		(job.job_kind == 'script' || isScriptPreview(job?.job_kind))
	)
		start(job)

	function start(job: Job) {
		busy = true
		startedAt = new Date(job?.started_at ?? '').getTime()
		interval = setInterval(updateDuration, updateResolution)
	}

	function updateDuration() {
		if (job?.type == 'CompletedJob') {
			clearInterval(interval)
			return
		}

		if (startedAt) executionDuration = Date.now() - startedAt

		// Detect long running
		if (executionDuration >= longDefinition) longRunning = true
	}

	onDestroy(() => {
		// Clear the interval when the component is destroyed
		clearInterval(interval)
	})
</script>
