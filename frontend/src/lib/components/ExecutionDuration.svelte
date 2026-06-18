<script lang="ts">
	import { run } from 'svelte/legacy';

	import { type Job } from '$lib/gen'
	import { isScriptPreview } from '$lib/utils'
	import { onDestroy } from 'svelte'

	
	
	
	
	interface Props {
		job?: Job | undefined;
		/** Execution duration of current active job (in ms) */
		executionDuration?: number;
		/** Is current job running more than specified value in `longDefinition` seconds */
		longRunning?: boolean;
		/** What do we count as "long" (in ms)*/
		longDefinition?: number;
		/** How often component updates execution duration (in ms)
	 *   Higher value -> more efficient component is, less accuracy it has
	 *   Lower value -> less efficient component is, more accuracy it has
	 */
		updateResolution?: number;
	}

	let {
		job = undefined,
		executionDuration = $bindable(0),
		longRunning = $bindable(false),
		longDefinition = 30_000,
		updateResolution = 5_000
	}: Props = $props();

	let startedAt: number | undefined = undefined
	let busy: boolean = $state(false)
	let interval: number | undefined

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
	// Detect when execution of job started
	run(() => {
		if (
			!busy &&
			job &&
			'running' in job &&
			(job.job_kind == 'script' || isScriptPreview(job?.job_kind))
		)
			start(job)
	});
</script>
