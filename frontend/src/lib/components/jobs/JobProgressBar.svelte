<script lang="ts">
	import { type Job } from '$lib/gen'
	import ProgressBar from '../progressBar/ProgressBar.svelte'

	export let job: Job | undefined = undefined
	export let compact: boolean = false;
	/// Progress of currently running job
	export let scriptProgress: number | undefined = undefined;
	// Removes `Step 1` and replaces it with `Running` 
	export let hideStepTitle: boolean = false

	let error: number | undefined = undefined
	let index = 0
	let subIndex: number = 0
	let subLength: number  = 100
	let length = 1
	let nextInProgress = false

	$: if (job) updateJobProgress(job);
	$: subIndex = scriptProgress ?? 0;

	function updateJobProgress(job: Job) { 
		if (job['success'])	
			index = 1, subLength = 0, subIndex = 0;				
	}

	let resetP: any

	export function reset() {
		resetP?.()
		error = undefined
		subIndex = 0
		subLength = 100
		length = 1
		index = 0
		scriptProgress = undefined
	}

</script>

{#if scriptProgress}
<ProgressBar
	bind:resetP
	{length}
	{index}
	{nextInProgress}
	{subLength}
	{subIndex}
	{error}
	class={$$props.class}
	bind:compact
	bind:hideStepTitle
/>
{/if}
