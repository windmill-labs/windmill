<script lang="ts">
	import { type Job, MetricsService } from '$lib/gen'
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
		if (!job['running'] && !job['success']){
			error = 0;			
		}	else {
			error = undefined;			
		}
		
		if (job['success'])	
			index = 1, subLength = 0, subIndex = 0, scriptProgress = 100;				
		else if (job.type == "CompletedJob"){
			// If error occured and job is completed
			// than we fetch progress from server to display on what progress did it fail
			// Works on [...run] route. Could be displayed after run or as a historical page
			// If opening page without running it (e.g. reloading page after run) progress will be displayed instantly
			MetricsService.getJobProgress({
				workspace: job.workspace_id ?? "NO_WORKSPACE",
				id: job.id,
			}).then(progress => {
				scriptProgress = progress;
			});
		}
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
