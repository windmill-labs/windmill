<script lang="ts">
	// Efficient way to track execution time of jobs
	import { type Job } from '$lib/gen'
	import { onDestroy } from 'svelte'

	export let job: Job | undefined = undefined
	/** Pure execution duration of current active job
	 *  If job is flow, variable will represent duration of currently active subjob
	 *  It will count only level-0 subjobs, meaning subjobs of subflows will be ignored
	 *  executionDuration is guranteed to be started when job starts execution
	 */
	export let executionDuration: number = 0
	/** Is current job running more than specified value in `longDefinition` seconds */
	export let longRunning: boolean = false
	/** What do we count as "long" (in seconds)*/
	export let longDefinition: number = 3
	/** How often component updates execution duration (in seconds)
	 *   Higher value -> more efficient component is, less accuracy it has
	 *   Lower value -> less efficient component is, more accuracy it has
	 */
	export let updateResolution: number = 2

	// Use this internally and write results to exported executionDuration
	let _executionDuration: number = 0
	// Track job
	let trackedJob: string | undefined = undefined
	// In case of flows (flow_status.step)
	let trackedIndex: number | undefined = undefined

	let interval: NodeJS.Timeout | undefined
	// Detect when execution of job started
	$: if (
		(trackedIndex == undefined || trackedIndex != job?.flow_status?.step) &&
		!trackedJob &&
		job
	)
		tryStart(job)

	// We want to start timer just right after job execution started
	function tryStart(job: Job) {
		console.log('Update in job')
		// Handle individual jobs
		if (job.job_kind == 'script' || job.job_kind == 'preview') {
			// It is possible that this function is invoked multiple times on single job
			if (trackedJob != job.id && job['running']) {
				trackedJob = job.id
				start()
			}
		}

		// Handle flows
		else if (
			job.flow_status &&
			['flow', 'flowpreview', 'singlescriptflow'].includes(job.job_kind)
		) {
			const status = job.flow_status
			let module = status.modules[status?.step]

			if (trackedIndex != status.step && module.type == 'InProgress' ) {
				_executionDuration = 0
				trackedIndex = status.step
				longRunning = false
				// We want to track every subjob right after beginning of their execution
				start()
			}
		}
		// Other job kinds are not supported
		else
			console.warn(
				"JobKind of '",
				job.job_kind,
				"' is not supported by ExecutionDuration component"
			)
	}

	function start() {
		// If there is any running intervals we want to make sure we stop them
		clearInterval(interval)
		interval = setInterval(updateDuration, updateResolution * 1_000)
	}

	function updateDuration() {
		if (job?.type == 'CompletedJob') {
			clearInterval(interval)
			return
		}

		_executionDuration += updateResolution

		// Detect long running
		if (_executionDuration >= longDefinition) longRunning = true

		// Update exposed executionDuration
		executionDuration = _executionDuration
	}

	onDestroy(() => {
		// Clear the interval when the component is destroyed
		clearInterval(interval)
	})
</script>
