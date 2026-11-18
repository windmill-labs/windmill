<script lang="ts">
	import { type Job } from '$lib/gen'
	import ProgressBar from '../progressBar/ProgressBar.svelte'

	interface Props {
		job?: Job | undefined
		compact?: boolean
		/// Progress of currently running job
		scriptProgress?: number | undefined
		// Removes `Step 1` and replaces it with `Running`
		hideStepTitle?: boolean
		class?: string
	}

	let {
		job = undefined,
		compact = false,
		scriptProgress = undefined,
		hideStepTitle = false,
		class: className = ''
	}: Props = $props()

	let error: number | undefined = $state(undefined)
	let index = $state(0)
	let subIndex: number = $state(0)
	let subLength: number = $state(100)
	let length = $state(1)
	let nextInProgress = false
	let isCanceled = $state(false)
	let isScheduled = $state(false)

	let progressBar: ProgressBar | undefined = $state(undefined)
	let lastJobId = $state()

	function updateJobProgress(job: Job) {
		const completed = job.type === 'CompletedJob'
		isCanceled = job['canceled'] ?? false
		// A job still in the queue simply has not started: not running is only a failure
		// signal once the job is completed.
		isScheduled = !completed && !job['running']
		error = completed && !job['success'] && !isCanceled ? 0 : undefined
		// Anything that is success automatically gets 100% progress. Not gated on
		// `scriptProgress`: a job can complete without ever reporting progress, and the bar
		// would then stay on `Running` forever. A canceled job keeps the progress it reached.
		if (job['success']) ((index = 1), (subLength = 0), (subIndex = 0), (scriptProgress = 100))
	}

	export function reset() {
		progressBar?.resetP()
		error = undefined
		subIndex = 0
		subLength = 100
		length = 1
		index = 0
		isCanceled = false
		isScheduled = false
		scriptProgress = undefined
	}

	$effect(() => {
		if (lastJobId && job && job.id !== lastJobId) {
			lastJobId = job.id
			reset()
		}
	})

	$effect(() => {
		if (job) updateJobProgress(job)
	})
	$effect(() => {
		subIndex = scriptProgress ?? 0
	})
</script>

<ProgressBar
	bind:this={progressBar}
	{length}
	{index}
	{nextInProgress}
	{subLength}
	{subIndex}
	{error}
	class={className}
	{compact}
	{hideStepTitle}
	{isCanceled}
	{isScheduled}
/>
