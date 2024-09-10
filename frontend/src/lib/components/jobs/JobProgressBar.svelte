<script lang="ts">
	import { type Job } from '$lib/gen'
	import ProgressBar from '../progressBar/ProgressBar.svelte'

	export let job: Job | undefined = undefined
	/// Progress of currently running job
	export let progress: number | undefined = undefined;

	let error: number | undefined = undefined
	let index = 0
	let subIndex: number = 0
	let subLength: number  = 100
	let length = 1
	let nextInProgress = false

	$: if (job) updateJobProgress(job);
	$: subIndex = progress ?? 0;

  // TODO: Fix second run already 100%
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
		progress = undefined
	}

</script>

<style>
  :global(.compact-progress-bar > div:nth-child(2) ) {
  	border-radius: 0 !important;
  	height: 12px !important;
  }
  :global(.compact-progress-bar > div:nth-child(1)) {
  	display: none !important;
  }
</style>

{#if progress}
<ProgressBar
	bind:resetP
	{length}
	{index}
	{nextInProgress}
	{subLength}
	{subIndex}
	{error}
	class={$$props.class}
/>
{/if}
