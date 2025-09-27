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

	let progressBar: ProgressBar | undefined = $state(undefined)
	let lastJobId = $state()

	function updateJobProgress(job: Job) {
		if (!job['running'] && !job['success']) {
			error = 0
		} else {
			error = undefined
		}
		// Anything that is success automatically gets 100% progress
		if (job['success'] && scriptProgress)
			((index = 1), (subLength = 0), (subIndex = 0), (scriptProgress = 100))
	}

	export function reset() {
		progressBar?.resetP()
		error = undefined
		subIndex = 0
		subLength = 100
		length = 1
		index = 0
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
/>
