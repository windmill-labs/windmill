<script lang="ts">
	import { JobService, type Job, type ScriptArgs } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import Button from '../common/button/Button.svelte'
	import Toggle from '../Toggle.svelte'

	export let jobs: Job[] | undefined
	let modalIsOpen = true

	// local state
	let innerJobs: (Job & { priorArgs: ScriptArgs; isSelected: boolean })[] = []
	// $: innerJobs

	if (jobs) {
		const fewJobs = jobs.slice(0, 10)
		const promises = fewJobs.map((job) => JobService.getJobArgs({ id: job.id, workspace: 'demo' }))
		Promise.allSettled(promises)
			.then((r) => r.filter((result) => result.status === 'fulfilled'))
			.then((resArr) => resArr.map((p) => p.value) as ScriptArgs[])
			.then((r) => {
				innerJobs = fewJobs.map((job, i) => ({
					...job,
					priorArgs: r[i],
					args: structuredClone(r[i]),
					isSelected: false
				}))
			})
	}
</script>

<Modal
	bind:open={modalIsOpen}
	on:confirmed={() => {
		modalIsOpen = false
		// dispatch('confirmed')
	}}
	on:canceled
	title="Runnable batch"
	class="w-full max-w-full xl:max-w-7xl"
>
	<!-- <Toggle options={{ left: 'Select all' }} />
	<ul class=" list-disc pl-5">
		{#each jobsWithArgs as job}
			<li class="p-2 break-all">{JSON.stringify(job)}</li>
			<Toggle />
		{/each}
	</ul>

	<Button
		slot="actions"
		on:click={() => {
			open = false
			// dispatch('confirmed')
		}}
		color="light"
		size="sm"
	>
		<span class="inline-flex gap-2">Add something</span>
	</Button> -->
	<button on:click={() => console.log(innerJobs)}>console.log(jobsWithArgs)</button>
	<div class="flex space-x-6">
		<!-- Left Side: Job List -->
		<div class="w-1/3 p-4 border-r">
			<h3 class="text-lg font-semibold mb-4">Runnable Jobs</h3>
			<ul class="space-y-2">
				{#each innerJobs as job}
					<li>
						<Toggle bind:checked={job.isSelected} />
						<span>{job.id}</span>
						<span>{job.script_path}</span>
						<span>{job.tag}</span>
					</li>
				{/each}
			</ul>
		</div>

		<!-- Right Side: Job Configurations -->
		<div class="w-2/3 p-4">
			<h3 class="text-lg font-semibold mb-4">Job Configurations</h3>
			<Button
				on:click={() => {
					innerJobs = innerJobs.map((job) => ({
						...job,
						args: { ...job.priorArgs }
					}))
				}}
				color="light"
				size="sm">Reset job args to prior execution</Button
			>

			{#each innerJobs as job}
				{#if job.isSelected}
					<div class="mb-6">
						<h4 class="text-md font-semibold mb-2">{job.id} Config</h4>
						<div class="space-y-2">
							{#if job.args !== undefined}
								{#each Object.keys(job.args) as key}
									<div class="flex items-center space-x-4">
										<label class="w-20" for={key}>{key}</label>
										<input type="text" class="p-2 border rounded" bind:value={job.args[key]} />
									</div>
								{/each}
							{/if}
						</div>
					</div>
				{/if}
			{/each}
		</div>
	</div>
</Modal>
